use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MountVmHostAssignableDeviceInput {
    /// Device instance path in the host machine.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Location path to the assignable device.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostAssignableDeviceInfo {
    pub friendly_name: String,
    pub instance_path: String,
    pub location_path: String,
    /// Vendor ID as a string.
    pub vendor_id: String,
    /// Device ID as a string.
    pub device_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MountVmHostAssignableDeviceOutput {
    /// Devices that were mounted on the host.
    pub devices: Vec<VmHostAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct MountVmHostAssignableDeviceTool;

#[async_trait]
impl HyperVTool for MountVmHostAssignableDeviceTool {
    const NAME: &'static str = "hyperv_mount_vm_host_assignable_device";
    const DESCRIPTION: &'static str = "Mounts a device to a virtual machine (VM) host.";
    type Input = MountVmHostAssignableDeviceInput;
    type Output = MountVmHostAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.instance_path.is_none() && input.location_path.is_none() {
            return Err(ToolError::InvalidInput(
                "either instancePath or locationPath must be provided".to_string(),
            ));
        }

        if let Some(path) = &input.instance_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "instancePath must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(path) = &input.location_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "locationPath must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec!["Mount-VMHostAssignableDevice".to_string()];

        if let Some(instance_path) = &input.instance_path {
            args.push(format!(
                "-InstancePath '{}'",
                escape_ps_string(instance_path)
            ));
        }

        if let Some(location_path) = &input.location_path {
            args.push(format!(
                "-LocationPath '{}'",
                escape_ps_string(location_path)
            ));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Passthru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             FriendlyName, \
             InstancePath, \
             LocationPath, \
             @{{N='VendorId';E={{$_.VendorId.ToString()}}}}, \
             @{{N='DeviceId';E={{$_.DeviceId.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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
                vendor_id: device["VendorId"].as_str().unwrap_or_default().to_string(),
                device_id: device["DeviceId"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(MountVmHostAssignableDeviceOutput { devices: output })
    }
}

register_tool!(MountVmHostAssignableDeviceTool);
