use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmSwitchTeamMemberInput {
    /// Specifies the name of the virtual switch team to which to add members.
    pub team_name: String,
    /// Specifies the name of the network adapter to add to the virtual switch team.
    pub net_adapter_name: String,
    /// Specifies the name of the Hyper-V host. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmSwitchTeamMemberOutput {
    /// True if the cmdlet completed without sidecar error.
    pub success: bool,
}

#[derive(Default)]
pub struct AddVmSwitchTeamMemberTool;

#[async_trait]
impl HyperVTool for AddVmSwitchTeamMemberTool {
    const NAME: &'static str = "hyperv_add_vm_switch_team_member";
    const DESCRIPTION: &'static str = "Adds members to a virtual switch team.";
    type Input = AddVmSwitchTeamMemberInput;
    type Output = AddVmSwitchTeamMemberOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.team_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("team_name is required".to_string()));
        }
        if input.net_adapter_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "net_adapter_name is required".to_string(),
            ));
        }

        let mut args = vec!["Add-VMSwitchTeamMember".to_string()];
        args.push(format!(
            "-TeamName '{}'",
            escape_ps_string(&input.team_name)
        ));
        args.push(format!(
            "-NetAdapterName '{}'",
            escape_ps_string(&input.net_adapter_name)
        ));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object TeamName, NetAdapterName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        if let Some(obj) = raw.as_object() {
            if let Some(err) = obj.get("Error") {
                return Err(ToolError::Sidecar(
                    err.as_str()
                        .unwrap_or("Unknown PowerShell error")
                        .to_string(),
                ));
            }
        }

        Ok(AddVmSwitchTeamMemberOutput { success: true })
    }
}

register_tool!(AddVmSwitchTeamMemberTool);
