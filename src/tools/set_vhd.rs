use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVhdInput {
    /// Path to the virtual hard disk file whose properties are to be set.
    pub path: String,
    /// Hyper-V host on which the virtual hard disk resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Path to the parent disk of a differencing virtual hard disk.
    #[serde(default, rename = "parentPath")]
    pub parent_path: Option<String>,
    /// Path to the leaf virtual hard disk file in a differencing disk chain. Required when performing the operation in online mode.
    #[serde(default, rename = "leafPath")]
    pub leaf_path: Option<String>,
    /// Skip the identifier mismatch check between parent and child virtual hard disks.
    #[serde(default, rename = "ignoreIdMismatch")]
    pub ignore_id_mismatch: Option<bool>,
    /// Physical sector size in bytes. Valid values are 512 and 4096. Supported only on a VHDX-format disk that is not attached.
    #[serde(default, rename = "physicalSectorSizeBytes")]
    pub physical_sector_size_bytes: Option<u32>,
    /// Reset the disk identifier of the VHDX-format virtual disk.
    #[serde(default, rename = "resetDiskIdentifier")]
    pub reset_disk_identifier: Option<bool>,
    /// Force the command to run without asking for user confirmation.
    #[serde(default)]
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdInfo {
    pub path: String,
    pub attached: bool,
    pub block_size: String,
    pub computer_name: String,
    pub disk_identifier: String,
    pub file_size: String,
    pub is_deleted: bool,
    pub logical_sector_size: String,
    pub minimum_size: String,
    pub number: u32,
    pub parent_path: String,
    pub physical_sector_size: String,
    pub pool_name: String,
    pub size: String,
    pub vhd_format: String,
    pub vhd_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVhdOutput {
    pub vhds: Vec<VhdInfo>,
}

#[derive(Default)]
pub struct SetVhdTool;

#[async_trait]
impl HyperVTool for SetVhdTool {
    const NAME: &'static str = "hyperv_set_vhd";
    const DESCRIPTION: &'static str = "Sets properties associated with a virtual hard disk.";
    type Input = SetVhdInput;
    type Output = SetVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VHD path must not be empty".to_string(),
            ));
        }

        if input.parent_path.is_none()
            && input.physical_sector_size_bytes.is_none()
            && input.reset_disk_identifier.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one of parent_path, physical_sector_size_bytes, or reset_disk_identifier must be provided".to_string(),
            ));
        }

        let mut args = vec![format!("Set-VHD -Path '{}'", escape_ps_string(&input.path))];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(parent) = &input.parent_path {
            args.push(format!("-ParentPath '{}'", escape_ps_string(parent)));
        }
        if let Some(leaf) = &input.leaf_path {
            args.push(format!("-LeafPath '{}'", escape_ps_string(leaf)));
        }
        if input.ignore_id_mismatch == Some(true) {
            args.push("-IgnoreIdMismatch".to_string());
        }
        if let Some(size) = input.physical_sector_size_bytes {
            args.push(format!("-PhysicalSectorSizeBytes {}", size));
        }
        if input.reset_disk_identifier == Some(true) {
            args.push("-ResetDiskIdentifier".to_string());
        }
        if input.force == Some(true) {
            args.push("-Force".to_string());
        }

        let ps = format!(
            "{} | Select-Object Path, Attached, \
             @{{N='BlockSize';E={{$_.BlockSize.ToString()}}}}, \
             ComputerName, DiskIdentifier, \
             @{{N='FileSize';E={{$_.FileSize.ToString()}}}}, \
             IsDeleted, \
             @{{N='LogicalSectorSize';E={{$_.LogicalSectorSize.ToString()}}}}, \
             @{{N='MinimumSize';E={{$_.MinimumSize.ToString()}}}}, \
             Number, ParentPath, \
             @{{N='PhysicalSectorSize';E={{$_.PhysicalSectorSize.ToString()}}}}, \
             PoolName, \
             @{{N='Size';E={{$_.Size.ToString()}}}}, \
             @{{N='VHDFormat';E={{$_.VHDFormat.ToString()}}}}, \
             @{{N='VHDType';E={{$_.VHDType.ToString()}}}} | \
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

        let vhds = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(vhds.len());
        for vhd in vhds {
            output.push(VhdInfo {
                path: vhd["Path"].as_str().unwrap_or_default().to_string(),
                attached: vhd["Attached"].as_bool().unwrap_or_default(),
                block_size: vhd["BlockSize"].as_str().unwrap_or_default().to_string(),
                computer_name: vhd["ComputerName"].as_str().unwrap_or_default().to_string(),
                disk_identifier: vhd["DiskIdentifier"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                file_size: vhd["FileSize"].as_str().unwrap_or_default().to_string(),
                is_deleted: vhd["IsDeleted"].as_bool().unwrap_or_default(),
                logical_sector_size: vhd["LogicalSectorSize"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                minimum_size: vhd["MinimumSize"].as_str().unwrap_or_default().to_string(),
                number: vhd["Number"].as_u64().unwrap_or_default() as u32,
                parent_path: vhd["ParentPath"].as_str().unwrap_or_default().to_string(),
                physical_sector_size: vhd["PhysicalSectorSize"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                pool_name: vhd["PoolName"].as_str().unwrap_or_default().to_string(),
                size: vhd["Size"].as_str().unwrap_or_default().to_string(),
                vhd_format: vhd["VHDFormat"].as_str().unwrap_or_default().to_string(),
                vhd_type: vhd["VHDType"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SetVhdOutput { vhds: output })
    }
}

register_tool!(SetVhdTool);
