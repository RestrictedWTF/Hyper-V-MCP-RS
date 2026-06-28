use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmEventingInput {
    /// Hyper-V host on which to disable virtual machine eventing. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmEventingOutput {
    pub success: bool,
    #[serde(rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Default)]
pub struct DisableVmEventingTool;

#[async_trait]
impl HyperVTool for DisableVmEventingTool {
    const NAME: &'static str = "hyperv_disable_vm_eventing";
    const DESCRIPTION: &'static str = "Disables virtual machine eventing.";
    type Input = DisableVmEventingInput;
    type Output = DisableVmEventingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Disable-VMEventing".to_string()];
        args.push("-Force".to_string());

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = args.join(" ");

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let _raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        Ok(DisableVmEventingOutput {
            success: true,
            computer_name: input.computer_name,
        })
    }
}

register_tool!(DisableVmEventingTool);
