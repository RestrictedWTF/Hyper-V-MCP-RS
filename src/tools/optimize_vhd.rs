use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OptimizeVhdInput {
    /// Path to the dynamic or differencing virtual hard disk file to optimize.
    pub path: String,
    /// Optimization mode: Full, Quick, Retrim, Pretrimmed, or Prezeroed.
    #[serde(default)]
    pub mode: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdInfo {
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
    pub fragmentation_percentage: u32,
    pub alignment: u32,
    pub attached: bool,
    pub disk_identifier: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct OptimizeVhdOutput {
    pub vhds: Vec<VhdInfo>,
}

#[derive(Default)]
pub struct OptimizeVhdTool;

#[async_trait]
impl HyperVTool for OptimizeVhdTool {
    const NAME: &'static str = "hyperv_optimize_vhd";
    const DESCRIPTION: &'static str =
        "Optimizes the allocation of space used by virtual hard disk files, except for fixed virtual hard disks.";
    type Input = OptimizeVhdInput;
    type Output = OptimizeVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Optimize-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));

        if let Some(mode) = &input.mode {
            if mode.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Mode must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Mode '{}'", escape_ps_string(mode)));
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object \
             Path, \
             @{{N='VhdFormat';E={{$_.VhdFormat.ToString()}}}}, \
             @{{N='VhdType';E={{$_.VhdType.ToString()}}}}, \
             @{{N='FileSize';E={{if ($_.FileSize -eq $null) {{''}} else {{$_.FileSize.ToString()}}}}}}, \
             @{{N='Size';E={{if ($_.Size -eq $null) {{''}} else {{$_.Size.ToString()}}}}}}, \
             @{{N='MinimumSize';E={{if ($_.MinimumSize -eq $null) {{''}} else {{$_.MinimumSize.ToString()}}}}}}, \
             LogicalSectorSize, \
             PhysicalSectorSize, \
             @{{N='BlockSize';E={{if ($_.BlockSize -eq $null) {{''}} else {{$_.BlockSize.ToString()}}}}}}, \
             ParentPath, \
             FragmentationPercentage, \
             Alignment, \
             Attached, \
             DiskIdentifier | ConvertTo-Json -Compress -Depth 3",
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
                file_size: vhd["FileSize"].as_str().unwrap_or_default().to_string(),
                size: vhd["Size"].as_str().unwrap_or_default().to_string(),
                minimum_size: vhd["MinimumSize"].as_str().unwrap_or_default().to_string(),
                logical_sector_size: vhd["LogicalSectorSize"].as_u64().unwrap_or_default() as u32,
                physical_sector_size: vhd["PhysicalSectorSize"].as_u64().unwrap_or_default() as u32,
                block_size: vhd["BlockSize"].as_str().unwrap_or_default().to_string(),
                parent_path: vhd["ParentPath"].as_str().unwrap_or_default().to_string(),
                fragmentation_percentage: vhd["FragmentationPercentage"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                alignment: vhd["Alignment"].as_u64().unwrap_or_default() as u32,
                attached: vhd["Attached"].as_bool().unwrap_or_default(),
                disk_identifier: vhd["DiskIdentifier"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(OptimizeVhdOutput { vhds: output })
    }
}

register_tool!(OptimizeVhdTool);
