use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSystemSwitchExtensionSwitchFeatureInput {
    /// Name of the switch-level feature to retrieve.
    #[serde(default, rename = "featureName")]
    pub feature_name: Option<String>,
    /// Id of the switch-level feature to retrieve.
    #[serde(default, rename = "featureId")]
    pub feature_id: Option<String>,
    /// Name of the switch extension to which the feature belongs.
    #[serde(default, rename = "extensionName")]
    pub extension_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSystemSwitchExtensionSwitchFeatureInfo {
    pub id: String,
    pub extension_id: String,
    pub extension_name: String,
    pub name: String,
    pub is_deleted: bool,
    pub computer_name: String,
    pub setting_data: serde_json::Value,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSystemSwitchExtensionSwitchFeatureOutput {
    pub features: Vec<VmSystemSwitchExtensionSwitchFeatureInfo>,
}

#[derive(Default)]
pub struct GetVmSystemSwitchExtensionSwitchFeatureTool;

#[async_trait]
impl HyperVTool for GetVmSystemSwitchExtensionSwitchFeatureTool {
    const NAME: &'static str = "hyperv_get_vm_system_switch_extension_switch_feature";
    const DESCRIPTION: &'static str =
        "Gets the switch-level features on one or more Hyper-V hosts.";
    type Input = GetVmSystemSwitchExtensionSwitchFeatureInput;
    type Output = GetVmSystemSwitchExtensionSwitchFeatureOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSystemSwitchExtensionSwitchFeature".to_string()];

        if let Some(feature_name) = &input.feature_name {
            if feature_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Feature name must not be empty".to_string(),
                ));
            }
            args.push(format!("-FeatureName '{}'", escape_ps_string(feature_name)));
        }
        if let Some(feature_id) = &input.feature_id {
            let trimmed = feature_id.trim();
            if trimmed.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Feature id must not be empty".to_string(),
                ));
            }
            args.push(format!("-FeatureId '{}'", escape_ps_string(trimmed)));
        }
        if let Some(extension_name) = &input.extension_name {
            if extension_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Extension name must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-ExtensionName '{}'",
                escape_ps_string(extension_name)
            ));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             Id, ExtensionId, ExtensionName, Name, IsDeleted, ComputerName, SettingData | \
             ConvertTo-Json -Compress -Depth 10",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let features = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(features.len());
        for feature in features {
            output.push(VmSystemSwitchExtensionSwitchFeatureInfo {
                id: feature["Id"].as_str().unwrap_or_default().to_string(),
                extension_id: feature["ExtensionId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                extension_name: feature["ExtensionName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                name: feature["Name"].as_str().unwrap_or_default().to_string(),
                is_deleted: feature["IsDeleted"].as_bool().unwrap_or_default(),
                computer_name: feature["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                setting_data: feature
                    .get("SettingData")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null),
            });
        }

        Ok(GetVmSystemSwitchExtensionSwitchFeatureOutput { features: output })
    }
}

register_tool!(GetVmSystemSwitchExtensionSwitchFeatureTool);
