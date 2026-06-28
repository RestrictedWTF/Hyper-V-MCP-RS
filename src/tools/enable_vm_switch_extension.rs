use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnableVmSwitchExtensionInput {
    /// Name of the virtual switch.
    #[serde(rename = "vmSwitchName")]
    pub vm_switch_name: String,
    /// Name of the switch extension.
    #[serde(default, rename = "name")]
    pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct EnableVmSwitchExtensionOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct EnableVmSwitchExtensionTool;

#[async_trait]
impl HyperVTool for EnableVmSwitchExtensionTool {
    const NAME: &'static str = "hyperv_enable_vm_switch_extension";
    const DESCRIPTION: &'static str = "Enables one or more extensions on one or more switches.";
    type Input = EnableVmSwitchExtensionInput;
    type Output = EnableVmSwitchExtensionOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Enable-VMSwitchExtension".to_string()];
        if input.vm_switch_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_switch_name must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-VMSwitchName '{}'",
            escape_ps_string(&input.vm_switch_name)
        ));
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
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

        Ok(EnableVmSwitchExtensionOutput { success: true })
    }
}

register_tool!(EnableVmSwitchExtensionTool);
