use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionFeatureSetInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "featureId")]
    pub feature_id: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmSwitchExtensionSwitchFeatureInput {
    /// ID of the feature.
    #[serde(rename = "featureId")]
    pub feature_id: String,
    /// Name of the virtual switch.
    #[serde(rename = "vmSwitch")]
    pub vm_switch: String,
    /// Configuration data.
    #[serde(default, rename = "configData")]
    pub config_data: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmSwitchExtensionSwitchFeatureOutput {
    pub features: Vec<VmSwitchExtensionFeatureSetInfo>,
}

#[derive(Default)]
pub struct SetVmSwitchExtensionSwitchFeatureTool;

#[async_trait]
impl HyperVTool for SetVmSwitchExtensionSwitchFeatureTool {
    const NAME: &'static str = "hyperv_set_vm_switch_extension_switch_feature";
    const DESCRIPTION: &'static str = "Configures a feature on a virtual switch.";
    type Input = SetVmSwitchExtensionSwitchFeatureInput;
    type Output = SetVmSwitchExtensionSwitchFeatureOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMSwitchExtensionSwitchFeature".to_string()];
        if input.feature_id.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "feature_id must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-FeatureId '{}'",
            escape_ps_string(&input.feature_id)
        ));
        if input.vm_switch.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_switch must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-VMSwitch '{}'",
            escape_ps_string(&input.vm_switch)
        ));
        if let Some(config_data) = &input.config_data {
            if config_data.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "config_data must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ConfigData '{}'", escape_ps_string(config_data)));
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

        let ps = format!("{} | Select-Object Name, Id, FeatureId, Enabled, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            output.push(VmSwitchExtensionFeatureSetInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                feature_id: item["FeatureId"].as_str().unwrap_or_default().to_string(),
                enabled: item["Enabled"].as_bool().unwrap_or_default(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmSwitchExtensionSwitchFeatureOutput { features: output })
    }
}

register_tool!(SetVmSwitchExtensionSwitchFeatureTool);
