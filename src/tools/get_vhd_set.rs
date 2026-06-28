use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVhdSetInput {
    /// Path(s) to the VHD set file(s) to query.
    pub path: Vec<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Include the paths of all files on which the VHD set depends.
    #[serde(default, rename = "getAllPaths")]
    pub get_all_paths: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VhdSetInfo {
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub path: String,
    #[serde(rename = "snapshotIdList")]
    pub snapshot_id_list: Vec<String>,
    #[serde(rename = "allPaths")]
    pub all_paths: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVhdSetOutput {
    pub vhd_sets: Vec<VhdSetInfo>,
}

#[derive(Default)]
pub struct GetVhdSetTool;

#[async_trait]
impl HyperVTool for GetVhdSetTool {
    const NAME: &'static str = "hyperv_get_vhd_set";
    const DESCRIPTION: &'static str = "Gets information about a VHD set.";
    type Input = GetVhdSetInput;
    type Output = GetVhdSetOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.is_empty() {
            return Err(ToolError::InvalidInput("path is required".to_string()));
        }

        let mut args = vec!["Get-VHDSet".to_string()];
        for path in &input.path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "path must not be empty".to_string(),
                ));
            }
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if input.get_all_paths == Some(true) {
            args.push("-GetAllPaths".to_string());
        }

        let ps = format!(
            "{} | Select-Object ComputerName, Path, SnapshotIdList, AllPaths | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let sets = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(sets.len());
        for set in sets {
            let strings_from = |value: &serde_json::Value| -> Vec<String> {
                value
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .map(|v| v.as_str().unwrap_or_default().to_string())
                            .collect()
                    })
                    .unwrap_or_default()
            };

            output.push(VhdSetInfo {
                computer_name: set["ComputerName"].as_str().unwrap_or_default().to_string(),
                path: set["Path"].as_str().unwrap_or_default().to_string(),
                snapshot_id_list: strings_from(&set["SnapshotIdList"]),
                all_paths: strings_from(&set["AllPaths"]),
            });
        }

        Ok(GetVhdSetOutput { vhd_sets: output })
    }
}

register_tool!(GetVhdSetTool);
