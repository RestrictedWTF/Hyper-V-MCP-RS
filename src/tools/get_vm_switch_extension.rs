use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmSwitchName")]
    pub vm_switch_name: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSwitchExtensionInput {
    /// Name of the virtual switch.
    #[serde(default, rename = "vmSwitchName")]
    pub vm_switch_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSwitchExtensionOutput {
    pub extensions: Vec<VmSwitchExtensionInfo>,
}

#[derive(Default)]
pub struct GetVmSwitchExtensionTool;

#[async_trait]
impl HyperVTool for GetVmSwitchExtensionTool {
    const NAME: &'static str = "hyperv_get_vm_switch_extension";
    const DESCRIPTION: &'static str = "Gets the extensions on one or more virtual switches.";
    type Input = GetVmSwitchExtensionInput;
    type Output = GetVmSwitchExtensionOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSwitchExtension".to_string()];
        if let Some(vm_switch_name) = &input.vm_switch_name {
            if vm_switch_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_switch_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-VMSwitchName '{}'",
                escape_ps_string(vm_switch_name)
            ));
        }
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

        let ps = format!("{} | Select-Object Name, Id, VMSwitchName, Enabled, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSwitchExtensionInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_switch_name: item["VMSwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                enabled: item["Enabled"].as_bool().unwrap_or_default(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmSwitchExtensionOutput { extensions: output })
    }
}

register_tool!(GetVmSwitchExtensionTool);
