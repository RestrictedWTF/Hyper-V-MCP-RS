use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewVmSwitchInput {
    /// Name of the new virtual switch.
pub name: String,
    /// Type of the virtual switch: External, Internal, or Private.
    #[serde(default, rename = "switchType")]
    pub switch_type: Option<String>,
    /// Name of the physical network adapter to bind to an external switch.
    #[serde(default, rename = "netAdapterName")]
    pub net_adapter_name: Option<String>,
    /// Interface description of the physical network adapter to bind to an external switch.
    #[serde(default, rename = "netAdapterInterfaceDescription")]
    pub net_adapter_interface_description: Option<String>,
    /// Specifies whether the management operating system is exposed to the virtual switch.
    #[serde(default, rename = "allowManagementOS")]
    pub allow_management_os: Option<bool>,
    /// Notes to associate with the virtual switch.
    #[serde(default)]
    pub notes: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchInfo {
    pub name: String,
    pub id: String,
    pub switch_type: String,
    pub net_adapter_interface_description: String,
    pub net_adapter_name: String,
    pub allow_management_os: bool,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct NewVmSwitchOutput {
    pub switches: Vec<VmSwitchInfo>,
}

#[derive(Default)]
pub struct NewVmSwitchTool;

#[async_trait]
impl HyperVTool for NewVmSwitchTool {
    const NAME: &'static str = "hyperv_new_vm_switch";
    const DESCRIPTION: &'static str =
        "Creates a new virtual switch on one or more virtual machine hosts.";
    type Input = NewVmSwitchInput;
    type Output = NewVmSwitchOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Switch name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["New-VMSwitch".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

        if let Some(switch_type) = &input.switch_type {
            if switch_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SwitchType must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-SwitchType '{}'", escape_ps_string(switch_type)));
        }
        if let Some(adapter_name) = &input.net_adapter_name {
            if adapter_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "NetAdapterName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-NetAdapterName '{}'",
                escape_ps_string(adapter_name)
            ));
        }
        if let Some(adapter_desc) = &input.net_adapter_interface_description {
            if adapter_desc.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "NetAdapterInterfaceDescription must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-NetAdapterInterfaceDescription '{}'",
                escape_ps_string(adapter_desc)
            ));
        }
        if let Some(allow) = input.allow_management_os {
            args.push(format!("-AllowManagementOS ${}", allow));
        }
        if let Some(notes) = &input.notes {
            args.push(format!("-Notes '{}'", escape_ps_string(notes)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} -PassThru | Select-Object Name, Id, \
             @{{N='SwitchType';E={{$_.SwitchType.ToString()}}}}, \
             NetAdapterInterfaceDescription, NetAdapterName, AllowManagementOS, ComputerName | \
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
                    .unwrap_or_default()
                    .to_string(),
                net_adapter_name: switch["NetAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                allow_management_os: switch["AllowManagementOS"].as_bool().unwrap_or_default(),
                computer_name: switch["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(NewVmSwitchOutput { switches: output })
    }
}

register_tool!(NewVmSwitchTool);
