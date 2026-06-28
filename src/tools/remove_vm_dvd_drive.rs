use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmDvdDriveInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Controller number of the DVD drive.
    #[serde(default, rename = "controllerNumber")]
    pub controller_number: Option<u32>,
    /// Controller location of the DVD drive.
    #[serde(default, rename = "controllerLocation")]
    pub controller_location: Option<u32>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmDvdDriveOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmDvdDriveTool;

#[async_trait]
impl HyperVTool for RemoveVmDvdDriveTool {
    const NAME: &'static str = "hyperv_remove_vm_dvd_drive";
    const DESCRIPTION: &'static str = "Deletes a DVD drive from a virtual machine.";
    type Input = RemoveVmDvdDriveInput;
    type Output = RemoveVmDvdDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMDvdDrive".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(controller_number) = &input.controller_number {
            args.push(format!("-ControllerNumber {}", controller_number));
        }
        if let Some(controller_location) = &input.controller_location {
            args.push(format!("-ControllerLocation {}", controller_location));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = args.join(" ");

        ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(RemoveVmDvdDriveOutput { success: true })
    }
}


register_tool!(RemoveVmDvdDriveTool);
