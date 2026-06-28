use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MountVhdInput {
    /// Path to the virtual hard disk file to mount.
    pub path: String,
    /// Mount without assigning drive letters to the volumes contained within the virtual hard disk.
    #[serde(default, rename = "noDriveLetter")]
    pub no_drive_letter: bool,
    /// Mount the virtual hard disk in read-only mode.
    #[serde(default)]
    pub read_only: bool,
    /// Unique ID of a VHD set snapshot to mount.
    #[serde(default, rename = "snapshotId")]
    pub snapshot_id: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MountedVhdInfo {
    pub path: String,
    pub vhd_format: String,
    pub vhd_type: String,
    pub size: String,
    pub file_size: String,
    pub attached: bool,
    pub disk_number: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MountVhdOutput {
    pub vhds: Vec<MountedVhdInfo>,
}

#[derive(Default)]
pub struct MountVhdTool;

#[async_trait]
impl HyperVTool for MountVhdTool {
    const NAME: &'static str = "hyperv_mount_vhd";
    const DESCRIPTION: &'static str = "Mounts one or more virtual hard disks.";
    type Input = MountVhdInput;
    type Output = MountVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Mount-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));

        if input.no_drive_letter {
            args.push("-NoDriveLetter".to_string());
        }
        if input.read_only {
            args.push("-ReadOnly".to_string());
        }
        if let Some(snapshot_id) = &input.snapshot_id {
            if snapshot_id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SnapshotId must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-SnapshotId '{}'", escape_ps_string(snapshot_id)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object Path, \
             @{{N='VhdFormat';E={{$_.VhdFormat.ToString()}}}}, \
             @{{N='VhdType';E={{$_.VhdType.ToString()}}}}, \
             @{{N='Size';E={{$_.Size.ToString()}}}}, \
             @{{N='FileSize';E={{$_.FileSize.ToString()}}}}, \
             Attached, DiskNumber | ConvertTo-Json -Compress -Depth 3",
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
            output.push(MountedVhdInfo {
                path: vhd["Path"].as_str().unwrap_or_default().to_string(),
                vhd_format: vhd["VhdFormat"].as_str().unwrap_or_default().to_string(),
                vhd_type: vhd["VhdType"].as_str().unwrap_or_default().to_string(),
                size: vhd["Size"].as_str().unwrap_or_default().to_string(),
                file_size: vhd["FileSize"].as_str().unwrap_or_default().to_string(),
                attached: vhd["Attached"].as_bool().unwrap_or_default(),
                disk_number: match &vhd["DiskNumber"] {
                    serde_json::Value::Null => None,
                    value => value.as_u64().map(|n| n as u32),
                },
            });
        }

        Ok(MountVhdOutput { vhds: output })
    }
}

register_tool!(MountVhdTool);
