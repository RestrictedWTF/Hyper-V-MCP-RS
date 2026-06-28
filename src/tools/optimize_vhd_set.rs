use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OptimizeVhdSetInput {
    /// Path to the VHD set file to optimize.
    pub path: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct OptimizeVhdSetOutput {
    pub paths: Vec<String>,
}

#[derive(Default)]
pub struct OptimizeVhdSetTool;

#[async_trait]
impl HyperVTool for OptimizeVhdSetTool {
    const NAME: &'static str = "hyperv_optimize_vhd_set";
    const DESCRIPTION: &'static str = "Optimizes VHD set files.";
    type Input = OptimizeVhdSetInput;
    type Output = OptimizeVhdSetOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let path = input.path;
        if path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Optimize-VHDSet".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&path)));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object Path | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut paths = Vec::with_capacity(items.len());
        for item in items {
            paths.push(item["Path"].as_str().unwrap_or_default().to_string());
        }

        Ok(OptimizeVhdSetOutput { paths })
    }
}

register_tool!(OptimizeVhdSetTool);
