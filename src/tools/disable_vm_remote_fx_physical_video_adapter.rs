use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmRemoteFxPhysicalVideoAdapterInput {
    /// Name of the RemoteFX physical video adapter.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmRemoteFxPhysicalVideoAdapterOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct DisableVmRemoteFxPhysicalVideoAdapterTool;

#[async_trait]
impl HyperVTool for DisableVmRemoteFxPhysicalVideoAdapterTool {
    const NAME: &'static str = "hyperv_disable_vm_remote_fx_physical_video_adapter";
    const DESCRIPTION: &'static str = "Disables one or more RemoteFX physical video adapters from use with RemoteFX-enabled virtual machines.";
    type Input = DisableVmRemoteFxPhysicalVideoAdapterInput;
    type Output = DisableVmRemoteFxPhysicalVideoAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Disable-VMRemoteFXPhysicalVideoAdapter".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "name must not be empty".to_string(),
            ));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
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

        Ok(DisableVmRemoteFxPhysicalVideoAdapterOutput { success: true })
    }
}

register_tool!(DisableVmRemoteFxPhysicalVideoAdapterTool);
