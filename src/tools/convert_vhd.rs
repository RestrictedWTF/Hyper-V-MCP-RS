use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConvertVhdInput {
    /// Path to the source virtual hard disk file to convert.
    pub path: String,
    /// Path for the converted virtual hard disk file.
    #[serde(rename = "destinationPath")]
    pub destination_path: String,
    /// Format of the converted virtual hard disk: VHD or VHDX.
    #[serde(default, rename = "vhdFormat")]
    pub vhd_format: Option<String>,
    /// Type of the converted virtual hard disk: Fixed, Dynamic, or Differencing.
    #[serde(default, rename = "vhdType")]
    pub vhd_type: Option<String>,
    /// Block size, in bytes, for the converted virtual hard disk.
    #[serde(default, rename = "blockSizeBytes")]
    pub block_size_bytes: Option<u32>,
    /// Path to the parent virtual hard disk for a differencing disk.
    #[serde(default, rename = "parentPath")]
    pub parent_path: Option<String>,
    /// Delete the source virtual hard disk after conversion.
    #[serde(default, rename = "deleteSource")]
    pub delete_source: Option<bool>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ConvertedVhdInfo {
    pub path: String,
    pub vhd_format: String,
    pub vhd_type: String,
    pub file_size: String,
    pub size: String,
    pub minimum_size: String,
    pub logical_sector_size: u32,
    pub physical_sector_size: u32,
    pub block_size: String,
    pub parent_path: String,
    pub disk_identifier: String,
    pub fragmentation_percentage: i32,
    pub attached: bool,
    pub disk_number: i32,
    pub is_deleted: bool,
    pub number: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ConvertVhdOutput {
    pub vhds: Vec<ConvertedVhdInfo>,
}

#[derive(Default)]
pub struct ConvertVhdTool;

#[async_trait]
impl HyperVTool for ConvertVhdTool {
    const NAME: &'static str = "hyperv_convert_vhd";
    const DESCRIPTION: &'static str =
        "Converts the format, version type, and block size of a virtual hard disk file.";
    type Input = ConvertVhdInput;
    type Output = ConvertVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Source Path must not be empty".to_string(),
            ));
        }
        if input.destination_path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "DestinationPath must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Convert-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
        args.push(format!(
            "-DestinationPath '{}'",
            escape_ps_string(&input.destination_path)
        ));

        if let Some(format) = &input.vhd_format {
            if format.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VHDFormat must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VHDFormat '{}'", escape_ps_string(format)));
        }
        if let Some(vhd_type) = &input.vhd_type {
            if vhd_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VHDType must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VHDType '{}'", escape_ps_string(vhd_type)));
        }
        if let Some(block_size) = input.block_size_bytes {
            args.push(format!("-BlockSizeBytes {}", block_size));
        }
        if let Some(parent) = &input.parent_path {
            if parent.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ParentPath must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ParentPath '{}'", escape_ps_string(parent)));
        }
        if input.delete_source == Some(true) {
            args.push("-DeleteSource".to_string());
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object Path, \
             @{{N='VhdFormat';E={{$_.VhdFormat.ToString()}}}}, \
             @{{N='VhdType';E={{$_.VhdType.ToString()}}}}, \
             @{{N='FileSize';E={{$_.FileSize.ToString()}}}}, \
             @{{N='Size';E={{$_.Size.ToString()}}}}, \
             @{{N='MinimumSize';E={{$_.MinimumSize.ToString()}}}}, \
             LogicalSectorSize, PhysicalSectorSize, \
             @{{N='BlockSize';E={{$_.BlockSize.ToString()}}}}, \
             ParentPath, DiskIdentifier, FragmentationPercentage, Attached, DiskNumber, IsDeleted, Number | ConvertTo-Json -Compress -Depth 3",
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
            output.push(ConvertedVhdInfo {
                path: vhd["Path"].as_str().unwrap_or_default().to_string(),
                vhd_format: vhd["VhdFormat"].as_str().unwrap_or_default().to_string(),
                vhd_type: vhd["VhdType"].as_str().unwrap_or_default().to_string(),
                file_size: vhd["FileSize"].as_str().unwrap_or_default().to_string(),
                size: vhd["Size"].as_str().unwrap_or_default().to_string(),
                minimum_size: vhd["MinimumSize"].as_str().unwrap_or_default().to_string(),
                logical_sector_size: vhd["LogicalSectorSize"].as_u64().unwrap_or_default() as u32,
                physical_sector_size: vhd["PhysicalSectorSize"].as_u64().unwrap_or_default() as u32,
                block_size: vhd["BlockSize"].as_str().unwrap_or_default().to_string(),
                parent_path: vhd["ParentPath"].as_str().unwrap_or_default().to_string(),
                disk_identifier: vhd["DiskIdentifier"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                fragmentation_percentage: vhd["FragmentationPercentage"]
                    .as_i64()
                    .unwrap_or_default() as i32,
                attached: vhd["Attached"].as_bool().unwrap_or_default(),
                disk_number: vhd["DiskNumber"].as_i64().unwrap_or_default() as i32,
                is_deleted: vhd["IsDeleted"].as_bool().unwrap_or_default(),
                number: vhd["Number"].as_i64().unwrap_or_default() as i32,
            });
        }

        Ok(ConvertVhdOutput { vhds: output })
    }
}

register_tool!(ConvertVhdTool);
