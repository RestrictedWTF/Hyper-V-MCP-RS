use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewVhdInput {
    /// Path to the new virtual hard disk file.
    pub path: String,
    /// Maximum size, in bytes, of the virtual hard disk.
    #[serde(rename = "sizeBytes")]
    pub size_bytes: u64,
    /// Creates a dynamically expanding virtual hard disk. This is the default if no type is specified.
    #[serde(default)]
    pub dynamic: Option<bool>,
    /// Creates a fixed-size virtual hard disk.
    #[serde(default)]
    pub fixed: Option<bool>,
    /// Creates a differencing virtual hard disk.
    #[serde(default)]
    pub differencing: Option<bool>,
    /// Path to the parent virtual hard disk for a differencing disk.
    #[serde(default, rename = "parentPath")]
    pub parent_path: Option<String>,
    /// Block size, in bytes, for the virtual hard disk.
    #[serde(default, rename = "blockSizeBytes")]
    pub block_size_bytes: Option<u32>,
    /// Logical sector size, in bytes, for the virtual hard disk.
    #[serde(default, rename = "logicalSectorSizeBytes")]
    pub logical_sector_size_bytes: Option<u32>,
    /// Physical sector size, in bytes, for the virtual hard disk.
    #[serde(default, rename = "physicalSectorSizeBytes")]
    pub physical_sector_size_bytes: Option<u32>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdInfo {
    pub path: String,
    pub size_bytes: String,
    pub file_size_bytes: String,
    pub block_size_bytes: String,
    pub hard_disk_type: String,
    pub logical_sector_size_bytes: String,
    pub physical_sector_size_bytes: String,
    pub parent_path: String,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct NewVhdOutput {
    pub vhds: Vec<VhdInfo>,
}

#[derive(Default)]
pub struct NewVhdTool;

#[async_trait]
impl HyperVTool for NewVhdTool {
    const NAME: &'static str = "hyperv_new_vhd";
    const DESCRIPTION: &'static str = "Creates one or more new virtual hard disks.";
    type Input = NewVhdInput;
    type Output = NewVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        let is_differencing = input.differencing.unwrap_or(false);
        let is_fixed = input.fixed.unwrap_or(false);
        let is_dynamic = input.dynamic.unwrap_or(false);

        if is_differencing {
            if input
                .parent_path
                .as_ref()
                .map(|p| p.trim().is_empty())
                .unwrap_or(true)
            {
                return Err(ToolError::InvalidInput(
                    "ParentPath is required when creating a differencing disk".to_string(),
                ));
            }
        } else if input.size_bytes == 0 {
            return Err(ToolError::InvalidInput(
                "SizeBytes must be greater than 0".to_string(),
            ));
        }

        let mut args = vec!["New-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));

        if !is_differencing {
            args.push(format!("-SizeBytes {}", input.size_bytes));
        }

        if is_dynamic {
            args.push("-Dynamic".to_string());
        } else if is_fixed {
            args.push("-Fixed".to_string());
        } else if is_differencing {
            args.push("-Differencing".to_string());
        }

        if let Some(parent_path) = &input.parent_path {
            if parent_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ParentPath must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ParentPath '{}'", escape_ps_string(parent_path)));
        }

        if let Some(block_size) = input.block_size_bytes {
            args.push(format!("-BlockSizeBytes {}", block_size));
        }
        if let Some(logical_sector_size) = input.logical_sector_size_bytes {
            args.push(format!("-LogicalSectorSizeBytes {}", logical_sector_size));
        }
        if let Some(physical_sector_size) = input.physical_sector_size_bytes {
            args.push(format!("-PhysicalSectorSizeBytes {}", physical_sector_size));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} -PassThru | Select-Object Path, \
             @{{N='SizeBytes';E={{$_.Size.ToString()}}}}, \
             @{{N='FileSizeBytes';E={{$_.FileSize.ToString()}}}}, \
             @{{N='BlockSizeBytes';E={{$_.BlockSize.ToString()}}}}, \
             @{{N='HardDiskType';E={{$_.HardDiskType.ToString()}}}}, \
             @{{N='LogicalSectorSizeBytes';E={{$_.LogicalSectorSize.ToString()}}}}, \
             @{{N='PhysicalSectorSizeBytes';E={{$_.PhysicalSectorSize.ToString()}}}}, \
             ParentPath, ComputerName | ConvertTo-Json -Compress -Depth 3",
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
                size_bytes: vhd["SizeBytes"].as_str().unwrap_or_default().to_string(),
                file_size_bytes: vhd["FileSizeBytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                block_size_bytes: vhd["BlockSizeBytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                hard_disk_type: vhd["HardDiskType"].as_str().unwrap_or_default().to_string(),
                logical_sector_size_bytes: vhd["LogicalSectorSizeBytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                physical_sector_size_bytes: vhd["PhysicalSectorSizeBytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_path: vhd["ParentPath"].as_str().unwrap_or_default().to_string(),
                computer_name: vhd["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(NewVhdOutput { vhds: output })
    }
}

register_tool!(NewVhdTool);
