use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopVmTraceInput {
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct StopVmTraceOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct StopVmTraceTool;

#[async_trait]
impl HyperVTool for StopVmTraceTool {
    const NAME: &'static str = "hyperv_stop_vm_trace";
    const DESCRIPTION: &'static str = "Stops tracing to file.";
    type Input = StopVmTraceInput;
    type Output = StopVmTraceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Stop-VMTrace".to_string()];
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = args.join(" ");

        ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(StopVmTraceOutput { success: true })
    }
}


register_tool!(StopVmTraceTool);
