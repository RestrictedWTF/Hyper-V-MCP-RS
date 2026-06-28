use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmSwitchExtensionPortFeatureInput {
    /// Name of the virtual machine whose network adapter receives the feature.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to which the feature is added.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Apply the feature to the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Name of the port-level feature to add.
    #[serde(rename = "featureName")]
    pub feature_name: String,
    /// Unique identifier (GUID) of the feature to add.
    #[serde(default, rename = "featureId")]
    pub feature_id: Option<String>,
    /// Name of the virtual switch extension that provides the feature.
    #[serde(default, rename = "extensionName")]
    pub extension_name: Option<String>,
    /// Unique identifier (GUID) of the virtual switch extension that provides the feature.
    #[serde(default, rename = "extensionId")]
    pub extension_id: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionPortFeatureInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "extensionId")]
    pub extension_id: String,
    #[serde(rename = "extensionName")]
    pub extension_name: String,
    #[serde(rename = "featureId")]
    pub feature_id: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmSwitchExtensionPortFeatureOutput {
    pub features: Vec<VmSwitchExtensionPortFeatureInfo>,
}

#[derive(Default)]
pub struct AddVmSwitchExtensionPortFeatureTool;

#[async_trait]
impl HyperVTool for AddVmSwitchExtensionPortFeatureTool {
    const NAME: &'static str = "hyperv_add_vm_switch_extension_port_feature";
    const DESCRIPTION: &'static str =
        "Adds a feature to a virtual network adapter.";
    type Input = AddVmSwitchExtensionPortFeatureInput;
    type Output = AddVmSwitchExtensionPortFeatureOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let feature_name = input.feature_name.trim();
        if feature_name.is_empty() {
            return Err(ToolError::InvalidInput(
                "Feature name must not be empty".to_string(),
            ));
        }

        let management_os = input.management_os == Some(true);
        if !management_os {
            match &input.vm_name {
                Some(vm) if !vm.trim().is_empty() => {}
                _ => {
                    return Err(ToolError::InvalidInput(
                        "VM name must be provided when management_os is not enabled".to_string(),
                    ));
                }
            }
        }

        let mut feature_args = vec!["New-VMSwitchExtensionPortFeature".to_string()];
        feature_args.push(format!("-FeatureName '{}'", escape_ps_string(feature_name)));

        if let Some(feature_id) = &input.feature_id {
            let feature_id = feature_id.trim();
            if !feature_id.is_empty() {
                feature_args.push(format!("-FeatureId '{}'", escape_ps_string(feature_id)));
            }
        }
        if let Some(extension_name) = &input.extension_name {
            let extension_name = extension_name.trim();
            if !extension_name.is_empty() {
                feature_args.push(format!(
                    "-ExtensionName '{}'",
                    escape_ps_string(extension_name)
                ));
            }
        }
        if let Some(extension_id) = &input.extension_id {
            let extension_id = extension_id.trim();
            if !extension_id.is_empty() {
                feature_args.push(format!(
                    "-ExtensionId '{}'",
                    escape_ps_string(extension_id)
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            let computer = computer.trim();
            if !computer.is_empty() {
                feature_args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
        }

        let mut add_args = vec!["Add-VMSwitchExtensionPortFeature".to_string()];

        if management_os {
            add_args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            add_args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        if let Some(adapter) = &input.vm_network_adapter_name {
            let adapter = adapter.trim();
            if !adapter.is_empty() {
                add_args.push(format!(
                    "-VMNetworkAdapterName '{}'",
                    escape_ps_string(adapter)
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            let computer = computer.trim();
            if !computer.is_empty() {
                add_args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
        }

        add_args.push("-VMSwitchExtensionPortFeature $feature".to_string());

        let ps = format!(
            "$feature = {}; {} | Select-Object \
             Name, Id, ExtensionId, ExtensionName, FeatureId, ComputerName, IsDeleted | \
             ConvertTo-Json -Compress -Depth 3",
            feature_args.join(" "),
            add_args.join(" ")
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
            output.push(VmSwitchExtensionPortFeatureInfo {
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
                feature_id: feature["FeatureId"]
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

        Ok(AddVmSwitchExtensionPortFeatureOutput { features: output })
    }
}

register_tool!(AddVmSwitchExtensionPortFeatureTool);
