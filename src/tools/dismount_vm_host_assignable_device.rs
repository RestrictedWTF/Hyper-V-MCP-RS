use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DismountVmHostAssignableDeviceInput {
    /// Device instance path in the host machine. Provide either instancePath or locationPath.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Location path to the assignable device. Provide either instancePath or locationPath.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Force the dismount without confirmation and bypass security checks.
    #[serde(default)]
    pub force: bool,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct HostAssignableDeviceInfo {
    pub friendly_name: String,
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "locationPath")]
    pub location_path: String,
    #[serde(rename = "vendorId")]
    pub vendor_id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DismountVmHostAssignableDeviceOutput {
    /// Devices that were dismounted from the host.
    pub dismounted: Vec<HostAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct DismountVmHostAssignableDeviceTool;

#[async_trait]
impl HyperVTool for DismountVmHostAssignableDeviceTool {
    const NAME: &'static str = "hyperv_dismount_vm_host_assignable_device";
    const DESCRIPTION: &'static str = "Dismounts a device from a virtual machine (VM) host.";
    type Input = DismountVmHostAssignableDeviceInput;
    type Output = DismountVmHostAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let has_instance = input
            .instance_path
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_location = input
            .location_path
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_instance && !has_location {
            return Err(ToolError::InvalidInput(
                "either instancePath or locationPath must be provided".to_string(),
            ));
        }

        let mut args = vec!["Dismount-VMHostAssignableDevice".to_string()];

        if let Some(path) = &input.instance_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "instancePath must not be empty".to_string(),
                ));
            }
            args.push(format!("-InstancePath '{}'", escape_ps_string(path)));
        }

        if let Some(path) = &input.location_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "locationPath must not be empty".to_string(),
                ));
            }
            args.push(format!("-LocationPath '{}'", escape_ps_string(path)));
        }

        if input.force {
            args.push("-Force".to_string());
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object FriendlyName, InstancePath, LocationPath, \
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

        let mut dismounted = Vec::with_capacity(devices.len());
        for dev in devices {
            dismounted.push(HostAssignableDeviceInfo {
                friendly_name: dev["FriendlyName"].as_str().unwrap_or_default().to_string(),
                instance_path: dev["InstancePath"].as_str().unwrap_or_default().to_string(),
                location_path: dev["LocationPath"].as_str().unwrap_or_default().to_string(),
                vendor_id: dev["VendorId"].as_str().unwrap_or_default().to_string(),
                device_id: dev["DeviceId"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(DismountVmHostAssignableDeviceOutput { dismounted })
    }
}

register_tool!(DismountVmHostAssignableDeviceTool);
