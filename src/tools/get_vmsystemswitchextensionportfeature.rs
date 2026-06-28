use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSystemSwitchExtensionPortFeatureInput {
    /// Name of the port-level feature to retrieve. If omitted, returns all features.
    #[serde(default, rename = "featureName")]
    pub feature_name: Option<String>,
    /// Unique identifier (GUID) of the feature to retrieve. If omitted, returns all features.
    #[serde(default, rename = "featureId")]
    pub feature_id: Option<String>,
    /// Name of the virtual switch extension for which features are to be retrieved.
    #[serde(default, rename = "extensionName")]
    pub extension_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSystemSwitchExtensionPortFeatureInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    #[serde(rename = "extensionName")]
    pub extension_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSystemSwitchExtensionPortFeatureOutput {
    pub features: Vec<VmSystemSwitchExtensionPortFeatureInfo>,
}

#[derive(Default)]
pub struct GetVmSystemSwitchExtensionPortFeatureTool;

#[async_trait]
impl HyperVTool for GetVmSystemSwitchExtensionPortFeatureTool {
    const NAME: &'static str = "hyperv_get_vmsystemswitchextensionportfeature";
    const DESCRIPTION: &'static str =
        "Gets the port-level features supported by virtual switch extensions on one or more Hyper-V hosts.";
    type Input = GetVmSystemSwitchExtensionPortFeatureInput;
    type Output = GetVmSystemSwitchExtensionPortFeatureOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSystemSwitchExtensionPortFeature".to_string()];

        if let Some(feature_name) = &input.feature_name {
            let feature_name = feature_name.trim();
            if feature_name.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Feature name must not be empty".to_string(),
                ));
            }
            args.push(format!("-FeatureName '{}'", escape_ps_string(feature_name)));
        }

        if let Some(feature_id) = &input.feature_id {
            let feature_id = feature_id.trim();
            if feature_id.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Feature ID must not be empty".to_string(),
                ));
            }
            args.push(format!("-FeatureId '{}'", escape_ps_string(feature_id)));
        }

        if let Some(extension_name) = &input.extension_name {
            let extension_name = extension_name.trim();
            if extension_name.is_empty() {
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
            let computer = computer.trim();
            if computer.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, Id, \
             ExtensionId, ExtensionName, ComputerName, IsDeleted | \
             ConvertTo-Json -Compress -Depth 3",
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
            output.push(VmSystemSwitchExtensionPortFeatureInfo {
                name: feature["Name"].as_str().unwrap_or_default().to_string(),
                id: feature["Id"].as_str().unwrap_or_default().to_string(),
                extension_id: feature["ExtensionId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                extension_name: feature["ExtensionName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: feature["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: feature["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmSystemSwitchExtensionPortFeatureOutput { features: output })
    }
}

register_tool!(GetVmSystemSwitchExtensionPortFeatureTool);
