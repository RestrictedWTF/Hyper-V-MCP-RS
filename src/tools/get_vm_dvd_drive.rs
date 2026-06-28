use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmDvdDriveInput {
    /// Name of the virtual machine whose DVD drives are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Controller number of the DVD drive to retrieve.
    #[serde(default, rename = "controllerNumber")]
    pub controller_number: Option<i32>,
    /// Controller location of the DVD drive to retrieve.
    #[serde(default, rename = "controllerLocation")]
    pub controller_location: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DvdDriveInfo {
    pub name: String,
    pub id: String,
    pub vm_name: String,
    pub vm_id: String,
    pub path: String,
    pub controller_number: i32,
    pub controller_location: i32,
    pub controller_type: String,
    pub dvd_media_type: String,
    pub pool_name: String,
    pub vm_snapshot_id: String,
    pub vm_snapshot_name: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmDvdDriveOutput {
    pub dvd_drives: Vec<DvdDriveInfo>,
}

#[derive(Default)]
pub struct GetVmDvdDriveTool;

#[async_trait]
impl HyperVTool for GetVmDvdDriveTool {
    const NAME: &'static str = "hyperv_get_vm_dvd_drive";
    const DESCRIPTION: &'static str =
        "Gets the DVD drives attached to a virtual machine or snapshot.";
    type Input = GetVmDvdDriveInput;
    type Output = GetVmDvdDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMDvdDrive".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(controller_number) = input.controller_number {
            args.push(format!("-ControllerNumber {}", controller_number));
        }
        if let Some(controller_location) = input.controller_location {
            args.push(format!("-ControllerLocation {}", controller_location));
        }

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

        Ok(GetVmDvdDriveOutput { dvd_drives: output })
    }
}

register_tool!(GetVmDvdDriveTool);
