use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVMSnapshotInput {
    /// Name of the virtual machine whose checkpoints are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the checkpoint to retrieve. If omitted, all checkpoints are returned.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Type of checkpoints to retrieve (e.g. Standard, Production).
    #[serde(default, rename = "snapshotType")]
    pub snapshot_type: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SnapshotInfo {
    pub name: String,
    pub id: String,
    pub vm_name: String,
    pub state: String,
    pub creation_time: String,
    pub checkpoint_type: String,
    pub parent_snapshot_id: String,
    pub parent_snapshot_name: String,
    pub checkpoint_file_location: String,
    pub size_of_system_files: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVMSnapshotOutput {
    pub snapshots: Vec<SnapshotInfo>,
}

#[derive(Default)]
pub struct GetVMSnapshotTool;

#[async_trait]
impl HyperVTool for GetVMSnapshotTool {
    const NAME: &'static str = "hyperv_get_vm_snapshot";
    const DESCRIPTION: &'static str =
        "Gets the checkpoints associated with a virtual machine or checkpoint.";
    type Input = GetVMSnapshotInput;
    type Output = GetVMSnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSnapshot".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Snapshot name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(snapshot_type) = &input.snapshot_type {
            if snapshot_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Snapshot type must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-SnapshotType '{}'",
                escape_ps_string(snapshot_type)
            ));
        }

        let ps = format!(
            "{} | Select-Object Name, Id, VMName, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='CreationTime';E={{$_.CreationTime.ToString()}}}}, \
             @{{N='CheckpointType';E={{$_.CheckpointType.ToString()}}}}, \
             ParentSnapshotId, ParentSnapshotName, CheckpointFileLocation, SizeOfSystemFiles | \
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

        let snapshots = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(snapshots.len());
        for snapshot in snapshots {
            output.push(SnapshotInfo {
                name: snapshot["Name"].as_str().unwrap_or_default().to_string(),
                id: snapshot["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: snapshot["VMName"].as_str().unwrap_or_default().to_string(),
                state: snapshot["State"].as_str().unwrap_or_default().to_string(),
                creation_time: snapshot["CreationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                checkpoint_type: snapshot["CheckpointType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_snapshot_id: snapshot["ParentSnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_snapshot_name: snapshot["ParentSnapshotName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                checkpoint_file_location: snapshot["CheckpointFileLocation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                size_of_system_files: snapshot["SizeOfSystemFiles"].as_u64().unwrap_or_default(),
            });
        }

        Ok(GetVMSnapshotOutput { snapshots: output })
    }
}

register_tool!(GetVMSnapshotTool);
