use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmHostAssignableDeviceInput {
    /// Name of the resource pool from which to remove the assignable device.
    #[serde(rename = "resourcePoolName")]
    pub resource_pool_name: String,
    /// Device instance path in the host machine.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Location path to the assignable device.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Suppress confirmation prompts.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedHostAssignableDeviceInfo {
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "locationPath")]
    pub location_path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmHostAssignableDeviceOutput {
    /// Assignable devices that were removed from the VM host.
    pub removed: Vec<RemovedHostAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct RemoveVmHostAssignableDeviceTool;

#[async_trait]
impl HyperVTool for RemoveVmHostAssignableDeviceTool {
    const NAME: &'static str = "hyperv_remove_vm_host_assignable_device";
    const DESCRIPTION: &'static str = "Removes a device assigned to a virtual machine (VM) host.";
    type Input = RemoveVmHostAssignableDeviceInput;
    type Output = RemoveVmHostAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let resource_pool_name = input.resource_pool_name.trim();
        if resource_pool_name.is_empty() {
            return Err(ToolError::InvalidInput(
                "resourcePoolName must not be empty".to_string(),
            ));
        }

        let instance_path_provided = input
            .instance_path
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let location_path_provided = input
            .location_path
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !instance_path_provided && !location_path_provided {
            return Err(ToolError::InvalidInput(
                "either instancePath or locationPath must be provided".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMHostAssignableDevice".to_string()];
        args.push(format!(
            "-ResourcePoolName '{}'",
            escape_ps_string(resource_pool_name)
        ));

        if let Some(instance_path) = &input.instance_path {
            let trimmed = instance_path.trim();
            if !trimmed.is_empty() {
                args.push(format!("-InstancePath '{}'", escape_ps_string(trimmed)));
            }
        }

        if let Some(location_path) = &input.location_path {
            let trimmed = location_path.trim();
            if !trimmed.is_empty() {
                args.push(format!("-LocationPath '{}'", escape_ps_string(trimmed)));
            }
        }

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
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object InstancePath, LocationPath | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let devices = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut removed = Vec::with_capacity(devices.len());
        for device in devices {
            removed.push(RemovedHostAssignableDeviceInfo {
                instance_path: device["InstancePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                location_path: device["LocationPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(RemoveVmHostAssignableDeviceOutput { removed })
    }
}

register_tool!(RemoveVmHostAssignableDeviceTool);
