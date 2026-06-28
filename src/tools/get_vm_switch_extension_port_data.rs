use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionPortDataInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "featureId")]
    pub feature_id: String,
    #[serde(rename = "portName")]
    pub port_name: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSwitchExtensionPortDataInput {
    /// ID of the feature.
    #[serde(default, rename = "featureId")]
    pub feature_id: Option<String>,
    /// Name of the port.
    #[serde(default, rename = "portName")]
    pub port_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSwitchExtensionPortDataOutput {
    pub data: Vec<VmSwitchExtensionPortDataInfo>,
}

#[derive(Default)]
pub struct GetVmSwitchExtensionPortDataTool;

#[async_trait]
impl HyperVTool for GetVmSwitchExtensionPortDataTool {
    const NAME: &'static str = "hyperv_get_vm_switch_extension_port_data";
    const DESCRIPTION: &'static str = "Retrieves the status of a virtual switch extension feature applied to a virtual network adapter.";
    type Input = GetVmSwitchExtensionPortDataInput;
    type Output = GetVmSwitchExtensionPortDataOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSwitchExtensionPortData".to_string()];
        if let Some(feature_id) = &input.feature_id {
            if feature_id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "feature_id must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-FeatureId '{}'", escape_ps_string(feature_id)));
        }
        if let Some(port_name) = &input.port_name {
            if port_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "port_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-PortName '{}'", escape_ps_string(port_name)));
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

        let ps = format!("{} | Select-Object Name, Id, FeatureId, PortName, Enabled, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSwitchExtensionPortDataInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                feature_id: item["FeatureId"].as_str().unwrap_or_default().to_string(),
                port_name: item["PortName"].as_str().unwrap_or_default().to_string(),
                enabled: item["Enabled"].as_bool().unwrap_or_default(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmSwitchExtensionPortDataOutput { data: output })
    }
}

register_tool!(GetVmSwitchExtensionPortDataTool);
