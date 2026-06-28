use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmDvdDriveInput {
    /// Name of the virtual machine to which the DVD drive is to be added.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Path to the ISO or virtual DVD file to attach. If omitted, an empty DVD drive is added.
    #[serde(default)]
    pub path: Option<String>,
    /// IDE controller number to which the DVD drive is attached.
    #[serde(default, rename = "controllerNumber")]
    pub controller_number: Option<i32>,
    /// Controller location on the IDE controller to which the DVD drive is attached.
    #[serde(default, rename = "controllerLocation")]
    pub controller_location: Option<i32>,
    /// Name of the resource pool from which the DVD drive is to be retrieved.
    #[serde(default, rename = "poolName")]
    pub pool_name: Option<String>,
    /// Allows the cmdlet to use a path that has not been verified.
    #[serde(default, rename = "allowUnverifiedPaths")]
    pub allow_unverified_paths: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DvdDriveInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    pub path: String,
    #[serde(rename = "controllerNumber")]
    pub controller_number: i32,
    #[serde(rename = "controllerLocation")]
    pub controller_location: i32,
    #[serde(rename = "controllerType")]
    pub controller_type: String,
    #[serde(rename = "dvdMediaType")]
    pub dvd_media_type: String,
    #[serde(rename = "poolName")]
    pub pool_name: String,
    #[serde(rename = "vmSnapshotId")]
    pub vm_snapshot_id: String,
    #[serde(rename = "vmSnapshotName")]
    pub vm_snapshot_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmDvdDriveOutput {
    pub drives: Vec<DvdDriveInfo>,
}

#[derive(Default)]
pub struct AddVmDvdDriveTool;

#[async_trait]
impl HyperVTool for AddVmDvdDriveTool {
    const NAME: &'static str = "hyperv_add_vm_dvd_drive";
    const DESCRIPTION: &'static str = "Adds a DVD drive to a virtual machine.";
    type Input = AddVmDvdDriveInput;
    type Output = AddVmDvdDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        if let Some(path) = &input.path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Path must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(pool_name) = &input.pool_name {
            if pool_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Pool name must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec![format!(
            "Add-VMDvdDrive -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(path) = &input.path {
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }
        if let Some(controller_number) = input.controller_number {
            args.push(format!("-ControllerNumber {}", controller_number));
        }
        if let Some(controller_location) = input.controller_location {
            args.push(format!("-ControllerLocation {}", controller_location));
        }
        if let Some(pool_name) = &input.pool_name {
            args.push(format!("-PoolName '{}'", escape_ps_string(pool_name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if input.allow_unverified_paths.unwrap_or(false) {
            args.push("-AllowUnverifiedPaths".to_string());
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, VMName, VMId, Path, ControllerNumber, ControllerLocation, \
             @{{N='ControllerType';E={{$_.ControllerType.ToString()}}}}, \
             @{{N='DvdMediaType';E={{$_.DvdMediaType.ToString()}}}}, \
             PoolName, VMSnapshotId, VMSnapshotName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
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
            output.push(DvdDriveInfo {
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                id: drive["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: drive["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: drive["VMId"].as_str().unwrap_or_default().to_string(),
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                controller_number: drive["ControllerNumber"].as_i64().unwrap_or_default() as i32,
                controller_location: drive["ControllerLocation"].as_i64().unwrap_or_default()
                    as i32,
                controller_type: drive["ControllerType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                dvd_media_type: drive["DvdMediaType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                pool_name: drive["PoolName"].as_str().unwrap_or_default().to_string(),
                vm_snapshot_id: drive["VMSnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_snapshot_name: drive["VMSnapshotName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: drive["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(AddVmDvdDriveOutput { drives: output })
    }
}

register_tool!(AddVmDvdDriveTool);
