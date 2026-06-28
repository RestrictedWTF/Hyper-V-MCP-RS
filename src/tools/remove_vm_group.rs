use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmGroupInput {
    /// Name of the virtual machine group.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmGroupOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmGroupTool;

#[async_trait]
impl HyperVTool for RemoveVmGroupTool {
    const NAME: &'static str = "hyperv_remove_vm_group";
    const DESCRIPTION: &'static str = "Removes a virtual machine group.";
    type Input = RemoveVmGroupInput;
    type Output = RemoveVmGroupOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMGroup".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
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

        Ok(RemoveVmGroupOutput { success: true })
    }
}


register_tool!(RemoveVmGroupTool);
