use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmHardDiskDriveInput {
    /// Name of the virtual machine from which to delete the hard disk drive.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Type of the controller where the hard disk is attached. Allowed values are IDE and SCSI.
    #[serde(rename = "controllerType")]
    pub controller_type: String,
    /// Number of the controller from which the hard disk drive is to be deleted.
    #[serde(rename = "controllerNumber")]
    pub controller_number: i32,
    /// Location on the controller from which the hard disk drive is to be deleted.
    #[serde(rename = "controllerLocation")]
    pub controller_location: i32,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedHardDiskDriveInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    pub path: String,
    #[serde(rename = "controllerType")]
    pub controller_type: String,
    #[serde(rename = "controllerNumber")]
    pub controller_number: i32,
    #[serde(rename = "controllerLocation")]
    pub controller_location: i32,
    #[serde(rename = "poolName")]
    pub pool_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmHardDiskDriveOutput {
    /// Hard disk drives that were removed.
    pub removed: Vec<RemovedHardDiskDriveInfo>,
}

#[derive(Default)]
pub struct RemoveVmHardDiskDriveTool;

#[async_trait]
impl HyperVTool for RemoveVmHardDiskDriveTool {
    const NAME: &'static str = "hyperv_remove_vm_hard_disk_drive";
    const DESCRIPTION: &'static str = "Deletes a hard disk drive from a virtual machine.";
    type Input = RemoveVmHardDiskDriveInput;
    type Output = RemoveVmHardDiskDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }

        let controller_type = input.controller_type.trim();
        if controller_type.is_empty() {
            return Err(ToolError::InvalidInput(
                "controllerType must not be empty".to_string(),
            ));
        }
        if !controller_type.eq_ignore_ascii_case("IDE")
            && !controller_type.eq_ignore_ascii_case("SCSI")
        {
            return Err(ToolError::InvalidInput(
                "controllerType must be IDE or SCSI".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMHardDiskDrive".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        args.push(format!(
            "-ControllerType '{}'",
            escape_ps_string(controller_type)
        ));
        args.push(format!("-ControllerNumber {}", input.controller_number));
        args.push(format!("-ControllerLocation {}", input.controller_location));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, VMName, Path, \
             @{{N='ControllerType';E={{$_.ControllerType.ToString()}}}}, \
             ControllerNumber, ControllerLocation, PoolName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let drives = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut removed = Vec::with_capacity(drives.len());
        for drive in drives {
            removed.push(RemovedHardDiskDriveInfo {
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                id: drive["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: drive["VMName"].as_str().unwrap_or_default().to_string(),
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                controller_type: drive["ControllerType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                controller_number: drive["ControllerNumber"].as_i64().unwrap_or_default() as i32,
                controller_location: drive["ControllerLocation"].as_i64().unwrap_or_default()
                    as i32,
                pool_name: drive["PoolName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(RemoveVmHardDiskDriveOutput { removed })
    }
}

register_tool!(RemoveVmHardDiskDriveTool);
