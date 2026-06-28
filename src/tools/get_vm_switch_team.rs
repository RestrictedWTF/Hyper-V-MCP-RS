use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchTeamInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "netAdapterNames")]
    pub net_adapter_names: Vec<String>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSwitchTeamInput {
    /// Name of the virtual switch team.
    #[serde(default, rename = "name")]
    pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSwitchTeamOutput {
    pub teams: Vec<VmSwitchTeamInfo>,
}


#[derive(Default)]
pub struct GetVmSwitchTeamTool;

#[async_trait]
impl HyperVTool for GetVmSwitchTeamTool {
    const NAME: &'static str = "hyperv_get_vm_switch_team";
    const DESCRIPTION: &'static str = "Gets virtual switch teams from Hyper-V hosts.";
    type Input = GetVmSwitchTeamInput;
    type Output = GetVmSwitchTeamOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSwitchTeam".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput("name must not be empty when provided".to_string()));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
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
            output.push(VmSwitchTeamInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                net_adapter_names,
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(GetVmSwitchTeamOutput { teams: output })

    }
}


register_tool!(GetVmSwitchTeamTool);
