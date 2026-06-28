use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmSwitchExtensionInput {
    /// Name of the virtual switch from which to disable the extension.
    #[serde(rename = "vmSwitchName")]
    pub vm_switch_name: String,
    /// Name of the switch extension to disable.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchExtensionInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "switchName")]
    pub switch_name: String,
    #[serde(rename = "extensionType")]
    pub extension_type: String,
    pub enabled: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmSwitchExtensionOutput {
    /// Switch extensions that were disabled.
    pub disabled: Vec<VmSwitchExtensionInfo>,
}

#[derive(Default)]
pub struct DisableVmSwitchExtensionTool;

#[async_trait]
impl HyperVTool for DisableVmSwitchExtensionTool {
    const NAME: &'static str = "hyperv_disable_vm_switch_extension";
    const DESCRIPTION: &'static str =
        "Disables one or more extensions on one or more virtual switches.";
    type Input = DisableVmSwitchExtensionInput;
    type Output = DisableVmSwitchExtensionOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_switch_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VMSwitchName must not be empty".to_string(),
            ));
        }
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Extension name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Disable-VMSwitchExtension".to_string()];
        args.push(format!(
            "-VMSwitchName '{}'",
            escape_ps_string(&input.vm_switch_name)
        ));
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

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
             Name, Id, SwitchName, \
             @{{N='ExtensionType';E={{$_.ExtensionType.ToString()}}}}, \
             Enabled, ComputerName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

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

        let mut disabled = Vec::with_capacity(items.len());
        for item in items {
            disabled.push(VmSwitchExtensionInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                switch_name: item["SwitchName"].as_str().unwrap_or_default().to_string(),
                extension_type: item["ExtensionType"]
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

        Ok(DisableVmSwitchExtensionOutput { disabled })
    }
}

register_tool!(DisableVmSwitchExtensionTool);
