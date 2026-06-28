use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSystemSwitchExtensionInput {
    /// Name of the switch extension to retrieve. If omitted, returns all extensions.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSystemSwitchExtensionInfo {
    pub name: String,
    pub id: String,
    pub vendor: String,
    pub version: String,
    #[serde(rename = "extensionType")]
    pub extension_type: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSystemSwitchExtensionOutput {
    pub extensions: Vec<VmSystemSwitchExtensionInfo>,
}

#[derive(Default)]
pub struct GetVmSystemSwitchExtensionTool;

#[async_trait]
impl HyperVTool for GetVmSystemSwitchExtensionTool {
    const NAME: &'static str = "hyperv_get_vm_system_switch_extension";
    const DESCRIPTION: &'static str =
        "Gets the switch extensions installed on a virtual machine host.";
    type Input = GetVmSystemSwitchExtensionInput;
    type Output = GetVmSystemSwitchExtensionOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSystemSwitchExtension".to_string()];

        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Extension name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
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
             Name, Id, Vendor, Version, \
             @{{N='ExtensionType';E={{$_.ExtensionType.ToString()}}}}, \
             ComputerName, IsDeleted | \
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

        let extensions = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(extensions.len());
        for ext in extensions {
            output.push(VmSystemSwitchExtensionInfo {
                name: ext["Name"].as_str().unwrap_or_default().to_string(),
                id: ext["Id"].as_str().unwrap_or_default().to_string(),
                vendor: ext["Vendor"].as_str().unwrap_or_default().to_string(),
                version: ext["Version"].as_str().unwrap_or_default().to_string(),
                extension_type: ext["ExtensionType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: ext["ComputerName"].as_str().unwrap_or_default().to_string(),
                is_deleted: ext["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmSystemSwitchExtensionOutput { extensions: output })
    }
}

register_tool!(GetVmSystemSwitchExtensionTool);
