use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmVideoInput {
    /// Name of the virtual machine. If omitted, returns video settings for all VMs.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmVideoInfo {
    pub vm_name: String,
    pub vm_id: String,
    pub computer_name: String,
    pub name: String,
    pub id: String,
    pub resolution_type: String,
    pub horizontal_resolution: i32,
    pub vertical_resolution: i32,
    pub vm_snapshot_name: String,
    pub vm_snapshot_id: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmVideoOutput {
    pub videos: Vec<VmVideoInfo>,
}

#[derive(Default)]
pub struct GetVmVideoTool;

#[async_trait]
impl HyperVTool for GetVmVideoTool {
    const NAME: &'static str = "hyperv_get_vm_video";
    const DESCRIPTION: &'static str = "Gets video settings for virtual machines.";
    type Input = GetVmVideoInput;
    type Output = GetVmVideoOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMVideo".to_string()];
        if let Some(name) = &input.name {
            args.push(format!("-VMName '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             VMName, \
             @{{N='VMId';E={{$_.VMId.ToString()}}}}, \
             ComputerName, \
             Name, \
             Id, \
             @{{N='ResolutionType';E={{$_.ResolutionType.ToString()}}}}, \
             HorizontalResolution, \
             VerticalResolution, \
             VMSnapshotName, \
             @{{N='VMSnapshotId';E={{$_.VMSnapshotId.ToString()}}}}, \
             IsDeleted | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );
        // Note: VMId, VMSnapshotId are .NET Guid objects and ResolutionType is a
        // .NET enum. They are forced to strings via calculated Select-Object
        // properties so serde_json sees string values.

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let videos = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(videos.len());
        for video in videos {
            output.push(VmVideoInfo {
                vm_name: video["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: video["VMId"].as_str().unwrap_or_default().to_string(),
                computer_name: video["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                name: video["Name"].as_str().unwrap_or_default().to_string(),
                id: video["Id"].as_str().unwrap_or_default().to_string(),
                resolution_type: video["ResolutionType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                horizontal_resolution: video["HorizontalResolution"].as_i64().unwrap_or_default()
                    as i32,
                vertical_resolution: video["VerticalResolution"].as_i64().unwrap_or_default()
                    as i32,
                vm_snapshot_name: video["VMSnapshotName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_snapshot_id: video["VMSnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: video["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmVideoOutput { videos: output })
    }
}

register_tool!(GetVmVideoTool);
