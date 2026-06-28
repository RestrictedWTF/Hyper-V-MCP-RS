use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResetVmResourceMeteringInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the resource pool.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct ResetVmResourceMeteringOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct ResetVmResourceMeteringTool;

#[async_trait]
impl HyperVTool for ResetVmResourceMeteringTool {
    const NAME: &'static str = "hyperv_reset_vm_resource_metering";
    const DESCRIPTION: &'static str =
        "Resets the resource utilization data collected by Hyper-V resource metering.";
    type Input = ResetVmResourceMeteringInput;
    type Output = ResetVmResourceMeteringOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Reset-VMResourceMetering".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(resource_pool_name) = &input.resource_pool_name {
            if resource_pool_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "resource_pool_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ResourcePoolName '{}'",
                escape_ps_string(resource_pool_name)
            ));
        }
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

        Ok(ResetVmResourceMeteringOutput { success: true })
    }
}

register_tool!(ResetVmResourceMeteringTool);
