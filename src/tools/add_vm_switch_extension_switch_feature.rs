use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmSwitchExtensionSwitchFeatureInput {
    /// Name of the virtual switch to which the feature is to be added.
    #[serde(rename = "switchName")]
    pub switch_name: String,
    /// Name of the switch-level feature to add.
    #[serde(rename = "featureName")]
    pub feature_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionSwitchFeatureInfo {
    pub id: String,
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    #[serde(rename = "extensionName")]
    pub extension_name: String,
    pub name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "settingData")]
    pub setting_data: serde_json::Value,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmSwitchExtensionSwitchFeatureOutput {
    pub features: Vec<VmSwitchExtensionSwitchFeatureInfo>,
}

#[derive(Default)]
pub struct AddVmSwitchExtensionSwitchFeatureTool;

#[async_trait]
impl HyperVTool for AddVmSwitchExtensionSwitchFeatureTool {
    const NAME: &'static str = "hyperv_add_vm_switch_extension_switch_feature";
    const DESCRIPTION: &'static str = "Adds a feature to a virtual switch.";
    type Input = AddVmSwitchExtensionSwitchFeatureInput;
    type Output = AddVmSwitchExtensionSwitchFeatureOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let switch_name = input.switch_name.trim();
        if switch_name.is_empty() {
            return Err(ToolError::InvalidInput(
                "Switch name must not be empty".to_string(),
            ));
        }

        let feature_name = input.feature_name.trim();
        if feature_name.is_empty() {
            return Err(ToolError::InvalidInput(
                "Feature name must not be empty".to_string(),
            ));
        }

        let computer_clause = if let Some(computer) = &input.computer_name {
            let computer = computer.trim();
            if computer.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty when provided".to_string(),
                ));
            }
            format!(" -ComputerName '{}'", escape_ps_string(computer))
        } else {
            String::new()
        };

        let ps = format!(
            "$feature = Get-VMSystemSwitchExtensionSwitchFeature -FeatureName '{}'{} | Select-Object -First 1; \
             if ($feature -eq $null) {{ throw \"No switch extension switch feature named '{}' was found\" }}; \
             Add-VMSwitchExtensionSwitchFeature -SwitchName '{}'{} -VMSwitchExtensionFeature $feature -PassThru | \
             Select-Object Id, ExtensionId, ExtensionName, Name, IsDeleted, ComputerName, SettingData | \
             ConvertTo-Json -Compress -Depth 10",
            escape_ps_string(feature_name),
            computer_clause,
            escape_ps_string(feature_name),
            escape_ps_string(switch_name),
            computer_clause,
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
            output.push(VmSwitchExtensionSwitchFeatureInfo {
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

        Ok(AddVmSwitchExtensionSwitchFeatureOutput { features: output })
    }
}

register_tool!(AddVmSwitchExtensionSwitchFeatureTool);
