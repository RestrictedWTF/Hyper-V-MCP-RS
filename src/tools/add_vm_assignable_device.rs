use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmAssignableDeviceInput {
    /// Name of the virtual machine to which the assignable device is to be added.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Location path to the assignable device.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Device instance path in the host machine.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmAssignableDeviceInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "locationPath")]
    pub location_path: String,
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmAssignableDeviceOutput {
    pub devices: Vec<VmAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct AddVmAssignableDeviceTool;

#[async_trait]
impl HyperVTool for AddVmAssignableDeviceTool {
    const NAME: &'static str = "hyperv_add_vm_assignable_device";
    const DESCRIPTION: &'static str = "Adds an assignable device to a specific virtual machine.";
    type Input = AddVmAssignableDeviceInput;
    type Output = AddVmAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VMName must not be empty".to_string(),
            ));
        }

        if input.location_path.is_none() && input.instance_path.is_none() {
            return Err(ToolError::InvalidInput(
                "Either location_path or instance_path must be provided".to_string(),
            ));
        }

        if let Some(location_path) = &input.location_path {
            if location_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "LocationPath must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(instance_path) = &input.instance_path {
            if instance_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "InstancePath must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec![format!(
            "Add-VMAssignableDevice -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(location_path) = &input.location_path {
            args.push(format!(
                "-LocationPath '{}'",
                escape_ps_string(location_path)
            ));
        }
        if let Some(instance_path) = &input.instance_path {
            args.push(format!(
                "-InstancePath '{}'",
                escape_ps_string(instance_path)
            ));
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
            "{} | Select-Object VMName, LocationPath, InstancePath, ComputerName | ConvertTo-Json -Compress -Depth 3",
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

        let mut output = Vec::with_capacity(devices.len());
        for device in devices {
            output.push(VmAssignableDeviceInfo {
                vm_name: device["VMName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                location_path: device["LocationPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                instance_path: device["InstancePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: device["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(AddVmAssignableDeviceOutput { devices: output })
    }
}

register_tool!(AddVmAssignableDeviceTool);
