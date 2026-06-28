use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmSwitchExtensionSwitchFeatureInput {
    /// ID of the feature.
    #[serde(rename = "featureId")]
    pub feature_id: String,
    /// Name of the virtual switch.
    #[serde(default, rename = "vmSwitch")]
    pub vm_switch: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmSwitchExtensionSwitchFeatureOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmSwitchExtensionSwitchFeatureTool;

#[async_trait]
impl HyperVTool for RemoveVmSwitchExtensionSwitchFeatureTool {
    const NAME: &'static str = "hyperv_remove_vm_switch_extension_switch_feature";
    const DESCRIPTION: &'static str = "Removes a feature from a virtual switch.";
    type Input = RemoveVmSwitchExtensionSwitchFeatureInput;
    type Output = RemoveVmSwitchExtensionSwitchFeatureOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMSwitchExtensionSwitchFeature".to_string()];
        if input.feature_id.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "feature_id must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-FeatureId '{}'",
            escape_ps_string(&input.feature_id)
        ));
        if let Some(vm_switch) = &input.vm_switch {
            if vm_switch.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_switch must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VMSwitch '{}'", escape_ps_string(vm_switch)));
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

        Ok(RemoveVmSwitchExtensionSwitchFeatureOutput { success: true })
    }
}

register_tool!(RemoveVmSwitchExtensionSwitchFeatureTool);
