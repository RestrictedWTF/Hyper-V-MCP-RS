use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSwitchInput {
    /// Name of the virtual switch to retrieve. If omitted, returns all switches.
    #[serde(default)]
    pub name: Option<String>,
    /// Type of virtual switches to retrieve: External, Internal, or Private.
    #[serde(default, rename = "switchType")]
    pub switch_type: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "switchType")]
    pub switch_type: String,
    #[serde(rename = "netAdapterInterfaceDescription")]
    pub net_adapter_interface_description: Option<String>,
    #[serde(rename = "allowManagementOS")]
    pub allow_management_os: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSwitchOutput {
    pub switches: Vec<VmSwitchInfo>,
}

#[derive(Default)]
pub struct GetVmSwitchTool;

#[async_trait]
impl HyperVTool for GetVmSwitchTool {
    const NAME: &'static str = "hyperv_get_vm_switch";
    const DESCRIPTION: &'static str =
        "Gets virtual switches from one or more virtual Hyper-V hosts.";
    type Input = GetVmSwitchInput;
    type Output = GetVmSwitchOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSwitch".to_string()];

        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Switch name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }

        if let Some(switch_type) = &input.switch_type {
            let normalized = switch_type.trim();
            if normalized.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Switch type must not be empty".to_string(),
                ));
            }
            let valid = matches!(
                normalized.to_ascii_lowercase().as_str(),
                "external" | "internal" | "private"
            );
            if !valid {
                return Err(ToolError::InvalidInput(
                    "Switch type must be External, Internal, or Private".to_string(),
                ));
            }
            args.push(format!("-SwitchType '{}'", escape_ps_string(normalized)));
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
             Name, Id, \
             @{{N='SwitchType';E={{$_.SwitchType.ToString()}}}}, \
             NetAdapterInterfaceDescription, AllowManagementOS, ComputerName | \
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

        let switches = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(switches.len());
        for switch in switches {
            output.push(VmSwitchInfo {
                name: switch["Name"].as_str().unwrap_or_default().to_string(),
                id: switch["Id"].as_str().unwrap_or_default().to_string(),
                switch_type: switch["SwitchType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                net_adapter_interface_description: switch["NetAdapterInterfaceDescription"]
                    .as_str()
                    .map(String::from),
                allow_management_os: switch["AllowManagementOS"].as_bool().unwrap_or_default(),
                computer_name: switch["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmSwitchOutput { switches: output })
    }
}

register_tool!(GetVmSwitchTool);
