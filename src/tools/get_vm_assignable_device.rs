use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmAssignableDeviceInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "locationPath")]
    pub location_path: String,
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmAssignableDeviceInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmAssignableDeviceOutput {
    pub devices: Vec<VmAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct GetVmAssignableDeviceTool;

#[async_trait]
impl HyperVTool for GetVmAssignableDeviceTool {
    const NAME: &'static str = "hyperv_get_vm_assignable_device";
    const DESCRIPTION: &'static str =
        "Retrieves information about the assignable device from a specific virtual machine.";
    type Input = GetVmAssignableDeviceInput;
    type Output = GetVmAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMAssignableDevice".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
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

        let ps = format!("{} | Select-Object VMName, LocationPath, InstancePath, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmAssignableDeviceInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                location_path: item["LocationPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                instance_path: item["InstancePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmAssignableDeviceOutput { devices: output })
    }
}

register_tool!(GetVmAssignableDeviceTool);
