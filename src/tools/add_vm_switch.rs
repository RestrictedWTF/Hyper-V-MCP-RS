use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmSwitchInput {
    /// Name of the virtual switch to add to the Ethernet resource pool.
    pub name: String,
    /// Name of the Ethernet resource pool to which the virtual switch is added.
    #[serde(rename = "resourcePoolName")]
    pub resource_pool_name: String,
    /// Hyper-V host to target. Defaults to localhost.
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
    pub net_adapter_interface_description: String,
    #[serde(rename = "netAdapterName")]
    pub net_adapter_name: String,
    #[serde(rename = "allowManagementOS")]
    pub allow_management_os: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmSwitchOutput {
    pub switches: Vec<VmSwitchInfo>,
}

#[derive(Default)]
pub struct AddVmSwitchTool;

#[async_trait]
impl HyperVTool for AddVmSwitchTool {
    const NAME: &'static str = "hyperv_add_vm_switch";
    const DESCRIPTION: &'static str = "Adds a virtual switch to an Ethernet resource pool.";
    type Input = AddVmSwitchInput;
    type Output = AddVmSwitchOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Switch name must not be empty".to_string(),
            ));
        }
        if input.resource_pool_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Resource pool name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Add-VMSwitch".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        args.push(format!(
            "-ResourcePoolName '{}'",
            escape_ps_string(&input.resource_pool_name)
        ));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, \
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

        Ok(AddVmSwitchOutput { switches: output })
    }
}

register_tool!(AddVmSwitchTool);
