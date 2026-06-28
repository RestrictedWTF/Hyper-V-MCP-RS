use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmHostAssignableDeviceInput {
    /// Name of the resource pool to which the device is assigned.
    #[serde(rename = "resourcePoolName")]
    pub resource_pool_name: String,
    /// Location path to the assignable device.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Device instance path in the host machine.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Force the command to run without asking for user confirmation.
    #[serde(default)]
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostAssignableDeviceInfo {
    #[serde(rename = "friendlyName")]
    pub friendly_name: String,
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "locationPath")]
    pub location_path: String,
    #[serde(rename = "vendorId")]
    pub vendor_id: u32,
    #[serde(rename = "deviceId")]
    pub device_id: u32,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmHostAssignableDeviceOutput {
    pub devices: Vec<VmHostAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct AddVmHostAssignableDeviceTool;

#[async_trait]
impl HyperVTool for AddVmHostAssignableDeviceTool {
    const NAME: &'static str = "hyperv_add_vm_host_assignable_device";
    const DESCRIPTION: &'static str = "Adds an assignable device to a virtual machine (VM) host.";
    type Input = AddVmHostAssignableDeviceInput;
    type Output = AddVmHostAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.resource_pool_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "ResourcePoolName must not be empty".to_string(),
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
            "Add-VMHostAssignableDevice -ResourcePoolName '{}'",
            escape_ps_string(&input.resource_pool_name)
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
        if input.force.unwrap_or(false) {
            args.push("-Force".to_string());
        }

        let ps = format!(
            "{} | Select-Object FriendlyName, InstancePath, LocationPath, VendorId, DeviceId, ComputerName | ConvertTo-Json -Compress -Depth 3",
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
            output.push(VmHostAssignableDeviceInfo {
                friendly_name: device["FriendlyName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                instance_path: device["InstancePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                location_path: device["LocationPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vendor_id: device["VendorId"].as_u64().unwrap_or_default() as u32,
                device_id: device["DeviceId"].as_u64().unwrap_or_default() as u32,
                computer_name: device["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(AddVmHostAssignableDeviceOutput { devices: output })
    }
}

register_tool!(AddVmHostAssignableDeviceTool);
