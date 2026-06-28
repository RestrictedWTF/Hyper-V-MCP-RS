use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmGpuPartitionAdapterInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Instance path of the GPU partition adapter.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmGpuPartitionAdapterOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmGpuPartitionAdapterTool;

#[async_trait]
impl HyperVTool for RemoveVmGpuPartitionAdapterTool {
    const NAME: &'static str = "hyperv_remove_vm_gpu_partition_adapter";
    const DESCRIPTION: &'static str = "Removes an assigned GPU partition from a virtual machine.";
    type Input = RemoveVmGpuPartitionAdapterInput;
    type Output = RemoveVmGpuPartitionAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMGpuPartitionAdapter".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_name must not be empty".to_string(),
            ));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(instance_path) = &input.instance_path {
            if instance_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "instance_path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-InstancePath '{}'",
                escape_ps_string(instance_path)
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

        Ok(RemoveVmGpuPartitionAdapterOutput { success: true })
    }
}

register_tool!(RemoveVmGpuPartitionAdapterTool);
