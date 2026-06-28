use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVhdSnapshotInput {
    /// Path to the VHD set file from which to remove the checkpoint.
    pub path: String,
    /// Unique ID of the VHD checkpoint to remove.
    #[serde(rename = "snapshotId")]
    pub snapshot_id: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Persist an RCT-only reference point after deleting the checkpoint.
    #[serde(default, rename = "persistReferencePoint")]
    pub persist_reference_point: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdSnapshotInfo {
    pub path: String,
    #[serde(rename = "snapshotId")]
    pub snapshot_id: String,
    #[serde(rename = "parentSnapshotId")]
    pub parent_snapshot_id: String,
    #[serde(rename = "vhdSnapshotType")]
    pub vhd_snapshot_type: String,
    #[serde(rename = "creationTime")]
    pub creation_time: String,
    #[serde(rename = "sizeOfSystemFiles")]
    pub size_of_system_files: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVhdSnapshotOutput {
    /// Checkpoints that were removed from the VHD set file.
    pub removed: Vec<VhdSnapshotInfo>,
}

#[derive(Default)]
pub struct RemoveVhdSnapshotTool;

#[async_trait]
impl HyperVTool for RemoveVhdSnapshotTool {
    const NAME: &'static str = "hyperv_remove_vhd_snapshot";
    const DESCRIPTION: &'static str = "Removes a checkpoint from a VHD set file.";
    type Input = RemoveVhdSnapshotInput;
    type Output = RemoveVhdSnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "path must not be empty".to_string(),
            ));
        }
        if input.snapshot_id.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "snapshotId must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VHDSnapshot".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
        args.push(format!(
            "-SnapshotId '{}'",
            escape_ps_string(&input.snapshot_id)
        ));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if input.persist_reference_point {
            args.push("-PersistReferencePoint".to_string());
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object Path, SnapshotId, ParentSnapshotId, \
             @{{N='VhdSnapshotType';E={{$_.VhdSnapshotType.ToString()}}}}, \
             @{{N='CreationTime';E={{$_.CreationTime.ToString()}}}}, \
             SizeOfSystemFiles | ConvertTo-Json -Compress -Depth 3",
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

        let mut removed = Vec::with_capacity(snapshots.len());
        for snapshot in snapshots {
            removed.push(VhdSnapshotInfo {
                path: snapshot["Path"].as_str().unwrap_or_default().to_string(),
                snapshot_id: snapshot["SnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_snapshot_id: snapshot["ParentSnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vhd_snapshot_type: snapshot["VhdSnapshotType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                creation_time: snapshot["CreationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                size_of_system_files: snapshot["SizeOfSystemFiles"].as_u64().unwrap_or_default(),
            });
        }

        Ok(RemoveVhdSnapshotOutput { removed })
    }
}

register_tool!(RemoveVhdSnapshotTool);
