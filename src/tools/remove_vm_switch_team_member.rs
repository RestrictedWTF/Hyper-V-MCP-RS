use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmSwitchTeamMemberInput {
    /// Name of the virtual switch.
    #[serde(rename = "vmSwitch")]
    pub vm_switch: String,
    /// Name of the network adapter.
    #[serde(rename = "netAdapterName")]
    pub net_adapter_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmSwitchTeamMemberOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmSwitchTeamMemberTool;

#[async_trait]
impl HyperVTool for RemoveVmSwitchTeamMemberTool {
    const NAME: &'static str = "hyperv_remove_vm_switch_team_member";
    const DESCRIPTION: &'static str = "Removes a member from a virtual machine switch team.";
    type Input = RemoveVmSwitchTeamMemberInput;
    type Output = RemoveVmSwitchTeamMemberOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMSwitchTeamMember".to_string()];
        if input.vm_switch.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_switch must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-VMSwitch '{}'",
            escape_ps_string(&input.vm_switch)
        ));
        if input.net_adapter_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "net_adapter_name must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-NetAdapterName '{}'",
            escape_ps_string(&input.net_adapter_name)
        ));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = args.join(" ");

        ctx.sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(RemoveVmSwitchTeamMemberOutput { success: true })
    }
}

register_tool!(RemoveVmSwitchTeamMemberTool);
