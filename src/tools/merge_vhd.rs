use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MergeVhdInput {
    /// Path to the child virtual hard disk in the chain that is the source for the merge.
    pub path: String,
    /// Path to the child virtual hard disk in the chain that is the destination for the merge.
    #[serde(default, rename = "destinationPath")]
    pub destination_path: Option<String>,
    /// Run the cmdlet without prompting for confirmation.
    #[serde(default)]
    pub force: bool,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MergedVhdInfo {
    pub path: String,
    pub vhd_format: String,
    pub vhd_type: String,
    pub size: u64,
    pub file_size: u64,
    pub parent_path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MergeVhdOutput {
    pub vhds: Vec<MergedVhdInfo>,
}

#[derive(Default)]
pub struct MergeVhdTool;

#[async_trait]
impl HyperVTool for MergeVhdTool {
    const NAME: &'static str = "hyperv_merge_vhd";
    const DESCRIPTION: &'static str = "Merges virtual hard disks.";
    type Input = MergeVhdInput;
    type Output = MergeVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Merge-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));

        if let Some(destination_path) = &input.destination_path {
            if destination_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "DestinationPath must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-DestinationPath '{}'",
                escape_ps_string(destination_path)
            ));
        }

        if input.force {
            args.push("-Force".to_string());
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object Path, \
             @{{N='VhdFormat';E={{$_.VhdFormat.ToString()}}}}, \
             @{{N='VhdType';E={{$_.VhdType.ToString()}}}}, \
             Size, FileSize, ParentPath | ConvertTo-Json -Compress -Depth 3",
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
            output.push(MergedVhdInfo {
                path: vhd["Path"].as_str().unwrap_or_default().to_string(),
                vhd_format: vhd["VhdFormat"].as_str().unwrap_or_default().to_string(),
                vhd_type: vhd["VhdType"].as_str().unwrap_or_default().to_string(),
                size: vhd["Size"].as_u64().unwrap_or_default(),
                file_size: vhd["FileSize"].as_u64().unwrap_or_default(),
                parent_path: vhd["ParentPath"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(MergeVhdOutput { vhds: output })
    }
}

register_tool!(MergeVhdTool);
