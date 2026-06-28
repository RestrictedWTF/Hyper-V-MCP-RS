use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnableVmRemoteFxPhysicalVideoAdapterInput {
    /// Name of the RemoteFX physical video adapter.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct EnableVmRemoteFxPhysicalVideoAdapterOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct EnableVmRemoteFxPhysicalVideoAdapterTool;

#[async_trait]
impl HyperVTool for EnableVmRemoteFxPhysicalVideoAdapterTool {
    const NAME: &'static str = "hyperv_enable_vm_remote_fx_physical_video_adapter";
    const DESCRIPTION: &'static str = "Enables one or more RemoteFX physical video adapters for use with RemoteFX-enabled virtual machines.";
    type Input = EnableVmRemoteFxPhysicalVideoAdapterInput;
    type Output = EnableVmRemoteFxPhysicalVideoAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Enable-VMRemoteFXPhysicalVideoAdapter".to_string()];
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

        Ok(EnableVmRemoteFxPhysicalVideoAdapterOutput { success: true })
    }
}


register_tool!(EnableVmRemoteFxPhysicalVideoAdapterTool);
