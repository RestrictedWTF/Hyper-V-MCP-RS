use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmHardDiskDriveInput {
    /// Name of the virtual machine to which the hard disk drive is to be added.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Path to the virtual hard disk file to attach. Required unless DiskNumber is used.
    #[serde(default)]
    pub path: Option<String>,
    /// Type of controller to which the drive is attached: IDE or SCSI.
    #[serde(default, rename = "controllerType")]
    pub controller_type: Option<String>,
    /// Controller number to which the drive is attached.
    #[serde(default, rename = "controllerNumber")]
    pub controller_number: Option<i32>,
    /// Controller location to which the drive is attached.
    #[serde(default, rename = "controllerLocation")]
    pub controller_location: Option<i32>,
    /// Physical disk number of a pass-through disk to attach. Required unless Path is used.
    #[serde(default, rename = "diskNumber")]
    pub disk_number: Option<u32>,
    /// Name of the resource pool from which the virtual hard disk is to be retrieved.
    #[serde(default, rename = "poolName")]
    pub pool_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct HardDiskDriveInfo {
    pub vm_name: String,
    pub name: String,
    pub path: String,
    pub controller_type: String,
    pub controller_number: i32,
    pub controller_location: i32,
    pub disk_number: u32,
    pub pool_name: String,
    pub is_deleted: bool,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmHardDiskDriveOutput {
    pub drives: Vec<HardDiskDriveInfo>,
}

#[derive(Default)]
pub struct AddVmHardDiskDriveTool;

#[async_trait]
impl HyperVTool for AddVmHardDiskDriveTool {
    const NAME: &'static str = "hyperv_add_vm_hard_disk_drive";
    const DESCRIPTION: &'static str = "Adds a hard disk drive to a virtual machine.";
    type Input = AddVmHardDiskDriveInput;
    type Output = AddVmHardDiskDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VMName must not be empty".to_string(),
            ));
        }

        if input.path.is_none() && input.disk_number.is_none() && input.pool_name.is_none() {
            return Err(ToolError::InvalidInput(
                "At least one of path, disk_number, or pool_name must be provided".to_string(),
            ));
        }

        if let Some(path) = &input.path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Path must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec![format!(
            "Add-VMHardDiskDrive -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(path) = &input.path {
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }
        if let Some(controller_type) = &input.controller_type {
            if controller_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ControllerType must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ControllerType '{}'",
                escape_ps_string(controller_type)
            ));
        }
        if let Some(controller_number) = input.controller_number {
            args.push(format!("-ControllerNumber {}", controller_number));
        }
        if let Some(controller_location) = input.controller_location {
            args.push(format!("-ControllerLocation {}", controller_location));
        }
        if let Some(disk_number) = input.disk_number {
            args.push(format!("-DiskNumber {}", disk_number));
        }
        if let Some(pool_name) = &input.pool_name {
            if pool_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "PoolName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-PoolName '{}'", escape_ps_string(pool_name)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }


        let ps = format!(
            "{} | Select-Object \
             VMName, Name, Path, \
             @{{N='ControllerType';E={{$_.ControllerType.ToString()}}}}, \
             ControllerNumber, ControllerLocation, DiskNumber, PoolName, IsDeleted, ComputerName | \
             ConvertTo-Json -Compress -Depth 3",
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

        let mut output = Vec::with_capacity(drives.len());
        for drive in drives {
            output.push(HardDiskDriveInfo {
                vm_name: drive["VMName"].as_str().unwrap_or_default().to_string(),
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                controller_type: drive["ControllerType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                controller_number: drive["ControllerNumber"].as_i64().unwrap_or_default() as i32,
                controller_location: drive["ControllerLocation"].as_i64().unwrap_or_default()
                    as i32,
                disk_number: drive["DiskNumber"].as_u64().unwrap_or_default() as u32,
                pool_name: drive["PoolName"].as_str().unwrap_or_default().to_string(),
                is_deleted: drive["IsDeleted"].as_bool().unwrap_or_default(),
                computer_name: drive["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(AddVmHardDiskDriveOutput { drives: output })
    }
}

register_tool!(AddVmHardDiskDriveTool);
