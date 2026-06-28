use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchTeamSetInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "netAdapterNames")]
    pub net_adapter_names: Vec<String>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmSwitchTeamInput {
    /// Name of the virtual switch team.
pub name: String,
    /// Names of the network adapters.
    #[serde(rename = "netAdapterName")]
    pub net_adapter_name: Vec<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmSwitchTeamOutput {
    pub teams: Vec<VmSwitchTeamSetInfo>,
}


#[derive(Default)]
pub struct SetVmSwitchTeamTool;

#[async_trait]
impl HyperVTool for SetVmSwitchTeamTool {
    const NAME: &'static str = "hyperv_set_vm_switch_team";
    const DESCRIPTION: &'static str = "Configures a virtual switch team.";
    type Input = SetVmSwitchTeamInput;
    type Output = SetVmSwitchTeamOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMSwitchTeam".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if !input.net_adapter_name.is_empty() {
            let escaped: Vec<String> = input.net_adapter_name.iter().map(|a| format!("'{}'", escape_ps_string(a))).collect();
            args.push(format!("-NetAdapterName @({})", escaped.join(",")));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, NetAdapterNames, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            let net_adapter_names = match &item["NetAdapterNames"] {
                serde_json::Value::Array(arr) => arr.iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect(),
                _ => Vec::new(),
            };
            output.push(VmSwitchTeamSetInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                net_adapter_names,
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SetVmSwitchTeamOutput { teams: output })

    }
}


register_tool!(SetVmSwitchTeamTool);
