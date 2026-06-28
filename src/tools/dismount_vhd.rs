use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DismountVhdInput {
    /// Path(s) to the virtual hard disk file(s) to dismount.
    #[serde(default)]
    pub path: Option<Vec<String>>,
    /// Disk number of the virtual hard disk to dismount.
    #[serde(default, rename = "diskNumber")]
    pub disk_number: Option<u32>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Snapshot ID of a VHD set snapshot to dismount.
    #[serde(default, rename = "snapshotId")]
    pub snapshot_id: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DismountedVhdInfo {
    pub path: String,
    #[serde(rename = "diskNumber")]
    pub disk_number: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DismountVhdOutput {
    /// Virtual hard disks that were dismounted.
    pub dismounted_vhds: Vec<DismountedVhdInfo>,
}

#[derive(Default)]
pub struct DismountVhdTool;

#[async_trait]
impl HyperVTool for DismountVhdTool {
    const NAME: &'static str = "hyperv_dismount_vhd";
    const DESCRIPTION: &'static str = "Dismounts a virtual hard disk.";
    type Input = DismountVhdInput;
    type Output = DismountVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        match (&input.path, input.disk_number) {
            (None, None) => {
                return Err(ToolError::InvalidInput(
                    "either path or diskNumber must be provided".to_string(),
                ));
            }
            (Some(_), Some(_)) => {
                return Err(ToolError::InvalidInput(
                    "only one of path or diskNumber may be provided".to_string(),
                ));
            }
            _ => {}
        }

        let mut args = vec!["Dismount-VHD".to_string()];

        if let Some(paths) = &input.path {
            if paths.is_empty() {
                return Err(ToolError::InvalidInput(
                    "path list must not be empty".to_string(),
                ));
            }
            let mut paths_escaped = Vec::with_capacity(paths.len());
            for p in paths {
                if p.trim().is_empty() {
                    return Err(ToolError::InvalidInput(
                        "path entries must not be empty".to_string(),
                    ));
                }
                paths_escaped.push(format!("'{}'", escape_ps_string(p)));
            }
            args.push(format!("-Path {}", paths_escaped.join(",")));
        }

        if let Some(disk_number) = input.disk_number {
            args.push(format!("-DiskNumber {}", disk_number));
        }

        if let Some(snapshot_id) = &input.snapshot_id {
            if snapshot_id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "snapshotId must not be empty".to_string(),
                ));
            }
            args.push(format!("-SnapshotId '{}'", escape_ps_string(snapshot_id)));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object Path, DiskNumber | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let vhds = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(vhds.len());
        for vhd in vhds {
            output.push(DismountedVhdInfo {
                path: vhd["Path"].as_str().unwrap_or_default().to_string(),
                disk_number: vhd["DiskNumber"].as_u64().unwrap_or_default() as u32,
            });
        }

        Ok(DismountVhdOutput {
            dismounted_vhds: output,
        })
    }
}

register_tool!(DismountVhdTool);
