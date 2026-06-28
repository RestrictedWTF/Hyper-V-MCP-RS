use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartVmTraceInput {
    /// Path to the trace file.
    #[serde(rename = "filePath")]
    pub file_path: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct StartVmTraceOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct StartVmTraceTool;

#[async_trait]
impl HyperVTool for StartVmTraceTool {
    const NAME: &'static str = "hyperv_start_vm_trace";
    const DESCRIPTION: &'static str = "Starts tracing to a file.";
    type Input = StartVmTraceInput;
    type Output = StartVmTraceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Start-VMTrace".to_string()];
        if input.file_path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "file_path must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-FilePath '{}'",
            escape_ps_string(&input.file_path)
        ));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = args.join(" ");

        ctx.sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(StartVmTraceOutput { success: true })
    }
}

register_tool!(StartVmTraceTool);
