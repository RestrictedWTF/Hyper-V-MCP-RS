use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionSwitchDataInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "featureId")]
    pub feature_id: String,
    #[serde(rename = "vmSwitchName")]
    pub vm_switch_name: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSwitchExtensionSwitchDataInput {
    /// ID of the feature.
    #[serde(default, rename = "featureId")]
    pub feature_id: Option<String>,
    /// Name of the virtual switch.
    #[serde(default, rename = "vmSwitchName")]
    pub vm_switch_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSwitchExtensionSwitchDataOutput {
    pub data: Vec<VmSwitchExtensionSwitchDataInfo>,
}

#[derive(Default)]
pub struct GetVmSwitchExtensionSwitchDataTool;

#[async_trait]
impl HyperVTool for GetVmSwitchExtensionSwitchDataTool {
    const NAME: &'static str = "hyperv_get_vm_switch_extension_switch_data";
    const DESCRIPTION: &'static str =
        "Gets the status of a virtual switch extension feature applied on a virtual switch.";
    type Input = GetVmSwitchExtensionSwitchDataInput;
    type Output = GetVmSwitchExtensionSwitchDataOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSwitchExtensionSwitchData".to_string()];
        if let Some(feature_id) = &input.feature_id {
            if feature_id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "feature_id must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-FeatureId '{}'", escape_ps_string(feature_id)));
        }
        if let Some(vm_switch_name) = &input.vm_switch_name {
            if vm_switch_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_switch_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-VMSwitchName '{}'",
                escape_ps_string(vm_switch_name)
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

        let ps = format!("{} | Select-Object Name, Id, FeatureId, VMSwitchName, Enabled, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSwitchExtensionSwitchDataInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                feature_id: item["FeatureId"].as_str().unwrap_or_default().to_string(),
                vm_switch_name: item["VMSwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                enabled: item["Enabled"].as_bool().unwrap_or_default(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmSwitchExtensionSwitchDataOutput { data: output })
    }
}

register_tool!(GetVmSwitchExtensionSwitchDataTool);
