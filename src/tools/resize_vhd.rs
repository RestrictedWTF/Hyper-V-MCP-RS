use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResizeVhdInput {
    /// Path to the virtual hard disk file to resize.
    pub path: String,
    /// New size, in bytes, for the virtual hard disk.
    #[serde(rename = "sizeBytes")]
    pub size_bytes: Option<u64>,
    /// Shrink the virtual hard disk to its minimum possible size.
    #[serde(default)]
    pub to_minimum_size: bool,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdInfo {
    pub path: String,
    pub vhd_format: String,
    pub vhd_type: String,
    pub file_size: u64,
    pub size: u64,
    pub minimum_size: u64,
    pub logical_sector_size: u32,
    pub physical_sector_size: u32,
    pub block_size: u64,
    pub parent_path: String,
    pub disk_identifier: String,
    pub fragmentation_percentage: u32,
    pub attached: bool,
    pub disk_number: u32,
    pub number: u32,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ResizeVhdOutput {
    pub vhds: Vec<VhdInfo>,
}

#[derive(Default)]
pub struct ResizeVhdTool;

#[async_trait]
impl HyperVTool for ResizeVhdTool {
    const NAME: &'static str = "hyperv_resize_vhd";
    const DESCRIPTION: &'static str = "Resizes a virtual hard disk.";
    type Input = ResizeVhdInput;
    type Output = ResizeVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        match (input.size_bytes, input.to_minimum_size) {
            (Some(_), true) => {
                return Err(ToolError::InvalidInput(
                    "Specify either size_bytes or to_minimum_size, not both".to_string(),
                ));
            }
            (None, false) => {
                return Err(ToolError::InvalidInput(
                    "Either size_bytes or to_minimum_size must be specified".to_string(),
                ));
            }
            _ => {}
        }

        let mut args = vec!["Resize-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));

        if let Some(size) = input.size_bytes {
            args.push(format!("-SizeBytes {}", size));
        }

        if input.to_minimum_size {
            args.push("-ToMinimumSize".to_string());
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        // Return the resized VHD object so callers can verify the new size.
        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object \
             Path, \
             @{{N='VhdFormat';E={{$_.VhdFormat.ToString()}}}}, \
             @{{N='VhdType';E={{$_.VhdType.ToString()}}}}, \
             FileSize, Size, MinimumSize, LogicalSectorSize, PhysicalSectorSize, BlockSize, \
             ParentPath, DiskIdentifier, FragmentationPercentage, Attached, DiskNumber, Number, \
             IsDeleted | ConvertTo-Json -Compress -Depth 3",
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
            output.push(VhdInfo {
                path: vhd["Path"].as_str().unwrap_or_default().to_string(),
                vhd_format: vhd["VhdFormat"].as_str().unwrap_or_default().to_string(),
                vhd_type: vhd["VhdType"].as_str().unwrap_or_default().to_string(),
                file_size: vhd["FileSize"].as_u64().unwrap_or_default(),
                size: vhd["Size"].as_u64().unwrap_or_default(),
                minimum_size: vhd["MinimumSize"].as_u64().unwrap_or_default(),
                logical_sector_size: vhd["LogicalSectorSize"].as_u64().unwrap_or_default() as u32,
                physical_sector_size: vhd["PhysicalSectorSize"].as_u64().unwrap_or_default() as u32,
                block_size: vhd["BlockSize"].as_u64().unwrap_or_default(),
                parent_path: vhd["ParentPath"].as_str().unwrap_or_default().to_string(),
                disk_identifier: vhd["DiskIdentifier"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                fragmentation_percentage: vhd["FragmentationPercentage"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                attached: vhd["Attached"].as_bool().unwrap_or_default(),
                disk_number: vhd["DiskNumber"].as_u64().unwrap_or_default() as u32,
                number: vhd["Number"].as_u64().unwrap_or_default() as u32,
                is_deleted: vhd["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(ResizeVhdOutput { vhds: output })
    }
}

register_tool!(ResizeVhdTool);
