use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmSwitchInput {
    /// Name of the virtual switch to remove.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Suppress confirmation prompts.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedVmSwitchInfo {
    pub name: String,
    pub id: String,
    pub switch_type: String,
    pub net_adapter_interface_description: String,
    pub net_adapter_name: String,
    pub allow_management_os: bool,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmSwitchOutput {
    /// Virtual switches that were removed.
    pub removed: Vec<RemovedVmSwitchInfo>,
}

#[derive(Default)]
pub struct RemoveVmSwitchTool;

#[async_trait]
impl HyperVTool for RemoveVmSwitchTool {
    const NAME: &'static str = "hyperv_remove_vm_switch";
    const DESCRIPTION: &'static str = "Deletes a virtual switch.";
    type Input = RemoveVmSwitchInput;
    type Output = RemoveVmSwitchOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMSwitch".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if input.force {
            args.push("-Force".to_string());
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

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

        let mut removed = Vec::with_capacity(switches.len());
        for switch in switches {
            let name = switch["Name"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing Name in sidecar output".to_string())
            })?;
            let id = switch["Id"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing Id in sidecar output".to_string())
            })?;
            let switch_type = switch["SwitchType"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing SwitchType in sidecar output".to_string())
            })?;

            removed.push(RemovedVmSwitchInfo {
                name: name.to_string(),
                id: id.to_string(),
                switch_type: switch_type.to_string(),
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

        Ok(RemoveVmSwitchOutput { removed })
    }
}

register_tool!(RemoveVmSwitchTool);
