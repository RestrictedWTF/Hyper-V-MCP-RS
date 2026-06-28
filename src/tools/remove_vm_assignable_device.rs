use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmAssignableDeviceInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Location path of the assignable device.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmAssignableDeviceOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmAssignableDeviceTool;

#[async_trait]
impl HyperVTool for RemoveVmAssignableDeviceTool {
    const NAME: &'static str = "hyperv_remove_vm_assignable_device";
    const DESCRIPTION: &'static str =
        "Removes information about the assignable devices from a specific virtual machine.";
    type Input = RemoveVmAssignableDeviceInput;
    type Output = RemoveVmAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMAssignableDevice".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_name must not be empty".to_string(),
            ));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(location_path) = &input.location_path {
            if location_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "location_path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-LocationPath '{}'",
                escape_ps_string(location_path)
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

        let ps = args.join(" ");

        ctx.sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(RemoveVmAssignableDeviceOutput { success: true })
    }
}

register_tool!(RemoveVmAssignableDeviceTool);
