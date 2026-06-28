use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmFloppyDiskDriveInput {
    /// Name of the virtual machine whose floppy disk drives are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the snapshot whose floppy disk drives are to be retrieved.
    /// Requires vmName to be specified.
    #[serde(default, rename = "snapshotName")]
    pub snapshot_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FloppyDiskDriveInfo {
    pub name: String,
    pub id: String,
    pub path: String,
    pub vm_name: String,
    pub vm_id: String,
    pub computer_name: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmFloppyDiskDriveOutput {
    pub drives: Vec<FloppyDiskDriveInfo>,
}

#[derive(Default)]
pub struct GetVmFloppyDiskDriveTool;

#[async_trait]
impl HyperVTool for GetVmFloppyDiskDriveTool {
    const NAME: &'static str = "hyperv_get_vm_floppy_disk_drive";
    const DESCRIPTION: &'static str =
        "Gets the floppy disk drives of a virtual machine or snapshot.";
    type Input = GetVmFloppyDiskDriveInput;
    type Output = GetVmFloppyDiskDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let ps = if let Some(snapshot_name) = &input.snapshot_name {
            if snapshot_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Snapshot name must not be empty".to_string(),
                ));
            }
            let vm_name = match &input.vm_name {
                Some(name) if !name.trim().is_empty() => name,
                _ => {
                    return Err(ToolError::InvalidInput(
                        "VM name is required when snapshotName is provided".to_string(),
                    ));
                }
            };

            let mut snap_args = vec![
                "Get-VMSnapshot".to_string(),
                format!("-VMName '{}'", escape_ps_string(vm_name)),
            ];
            if let Some(computer) = &input.computer_name {
                snap_args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
            snap_args.push(format!("-Name '{}'", escape_ps_string(snapshot_name)));

            format!(
                "{} | Get-VMFloppyDiskDrive | Select-Object Name, Id, Path, VMName, VMId, ComputerName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
                snap_args.join(" ")
            )
        } else if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }

            let mut args = vec!["Get-VMFloppyDiskDrive".to_string()];
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
            if let Some(computer) = &input.computer_name {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }

            format!(
                "{} | Select-Object Name, Id, Path, VMName, VMId, ComputerName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
                args.join(" ")
            )
        } else {
            return Err(ToolError::InvalidInput(
                "Either vmName or snapshotName with vmName must be provided".to_string(),
            ));
        };

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
            output.push(FloppyDiskDriveInfo {
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                id: drive["Id"].as_str().unwrap_or_default().to_string(),
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                vm_name: drive["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: drive["VMId"].as_str().unwrap_or_default().to_string(),
                computer_name: drive["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: drive["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmFloppyDiskDriveOutput { drives: output })
    }
}

register_tool!(GetVmFloppyDiskDriveTool);
