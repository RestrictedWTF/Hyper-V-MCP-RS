use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmResourcePoolInput {
    /// Name of the resource pool.
pub name: String,
    /// Type of the resource pool.
    #[serde(rename = "resourcePoolType")]
    pub resource_pool_type: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmResourcePoolOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmResourcePoolTool;

#[async_trait]
impl HyperVTool for RemoveVmResourcePoolTool {
    const NAME: &'static str = "hyperv_remove_vm_resource_pool";
    const DESCRIPTION: &'static str = "Deletes a resource pool from one or more virtual machine hosts.";
    type Input = RemoveVmResourcePoolInput;
    type Output = RemoveVmResourcePoolOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMResourcePool".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if input.resource_pool_type.trim().is_empty() {
            return Err(ToolError::InvalidInput("resource_pool_type must not be empty".to_string()));
        }
        args.push(format!("-ResourcePoolType '{}'", escape_ps_string(&input.resource_pool_type)));
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

        Ok(RemoveVmResourcePoolOutput { success: true })
    }
}


register_tool!(RemoveVmResourcePoolTool);
