use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVhdInput {
    /// Path to the virtual hard disk file. If omitted, returns all VHDs known to Hyper-V.
    #[serde(default)]
    pub path: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
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
pub struct GetVhdOutput {
    pub vhds: Vec<VhdInfo>,
}

#[derive(Default)]
pub struct GetVhdTool;

#[async_trait]
impl HyperVTool for GetVhdTool {
    const NAME: &'static str = "hyperv_get_vhd";
    const DESCRIPTION: &'static str =
        "Gets the virtual hard disk object associated with a virtual hard disk.";
    type Input = GetVhdInput;
    type Output = GetVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VHD".to_string()];

        if let Some(path) = &input.path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Path must not be empty".to_string(),
                ));
            }
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

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

        Ok(GetVhdOutput { vhds: output })
    }
}

register_tool!(GetVhdTool);
