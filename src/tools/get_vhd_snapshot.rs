use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVhdSnapshotInput {
    /// Path to the VHD set file from which to get checkpoint information.
    pub path: String,
    /// Optional checkpoint ID to retrieve. If omitted, all checkpoints are returned.
    #[serde(default, rename = "snapshotId")]
    pub snapshot_id: Option<String>,
    /// Gets the paths of all files on which the VHD checkpoint depends.
    #[serde(default, rename = "getParentPaths")]
    pub get_parent_paths: Option<bool>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdSnapshotInfo {
    pub computer_name: String,
    pub file_path: String,
    pub snapshot_id: String,
    pub snapshot_path: String,
    pub creation_time: String,
    pub resilient_change_tracking_id: String,
    pub parent_paths_list: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVhdSnapshotOutput {
    pub snapshots: Vec<VhdSnapshotInfo>,
}

#[derive(Default)]
pub struct GetVhdSnapshotTool;

#[async_trait]
impl HyperVTool for GetVhdSnapshotTool {
    const NAME: &'static str = "hyperv_get_vhd_snapshot";
    const DESCRIPTION: &'static str = "Gets information about a checkpoint in a VHD set.";
    type Input = GetVhdSnapshotInput;
    type Output = GetVhdSnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let path = input.path.trim();
        if path.is_empty() {
            return Err(ToolError::InvalidInput(
                "VHD set path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Get-VHDSnapshot".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(path)));

        if let Some(snapshot_id) = &input.snapshot_id {
            if snapshot_id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Snapshot ID must not be empty".to_string(),
                ));
            }
            args.push(format!("-SnapshotId '{}'", escape_ps_string(snapshot_id)));
        }

        if input.get_parent_paths.unwrap_or(false) {
            args.push("-GetParentPaths".to_string());
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object ComputerName, FilePath, \
             @{{N='SnapshotId';E={{$_.SnapshotId.ToString()}}}}, \
             SnapshotPath, \
             @{{N='CreationTime';E={{$_.CreationTime.ToString()}}}}, \
             ResilientChangeTrackingId, ParentPathsList | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );
        // Note: SnapshotId is a .NET Guid and CreationTime is a DateTime.
        // They are forced to strings via calculated Select-Object properties so
        // serde_json sees string values.

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
            output.push(VhdSnapshotInfo {
                computer_name: snapshot["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                file_path: snapshot["FilePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                snapshot_id: snapshot["SnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                snapshot_path: snapshot["SnapshotPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                creation_time: snapshot["CreationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                resilient_change_tracking_id: snapshot["ResilientChangeTrackingId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_paths_list: match &snapshot["ParentPathsList"] {
                    serde_json::Value::Array(arr) => arr
                        .iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect(),
                    serde_json::Value::String(s) => vec![s.clone()],
                    _ => Vec::new(),
                },
            });
        }

        Ok(GetVhdSnapshotOutput { snapshots: output })
    }
}

register_tool!(GetVhdSnapshotTool);
