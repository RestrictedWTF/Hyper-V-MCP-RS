use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostAssignableDeviceInput {
    /// Device instance path to retrieve. If omitted, returns all assignable devices.
    #[serde(default, rename = "instancePath")]
    pub instance_path: Option<String>,
    /// Location path of the assignable device to retrieve. If omitted, returns all assignable devices.
    #[serde(default, rename = "locationPath")]
    pub location_path: Option<String>,
    /// Name of the resource pool to which the device is assigned.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostAssignableDeviceInfo {
    #[serde(rename = "instanceId")]
    pub instance_id: String,
    pub name: String,
    #[serde(rename = "friendlyName")]
    pub friendly_name: String,
    #[serde(rename = "locationPath")]
    pub location_path: String,
    #[serde(rename = "className")]
    pub class_name: String,
    pub vendor: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    pub description: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isGpu")]
    pub is_gpu: bool,
    #[serde(rename = "isSriov")]
    pub is_sriov: bool,
    #[serde(rename = "isDiscrete")]
    pub is_discrete: bool,
    #[serde(rename = "isDda")]
    pub is_dda: bool,
    #[serde(rename = "isVirtualFunction")]
    pub is_virtual_function: bool,
    #[serde(rename = "isShareable")]
    pub is_shareable: bool,
    #[serde(rename = "isDismounted")]
    pub is_dismounted: bool,
    #[serde(rename = "isNetAdapter")]
    pub is_net_adapter: bool,
    #[serde(rename = "isReadyForDda")]
    pub is_ready_for_dda: bool,
    #[serde(rename = "isInUse")]
    pub is_in_use: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostAssignableDeviceOutput {
    pub devices: Vec<VmHostAssignableDeviceInfo>,
}

#[derive(Default)]
pub struct GetVmHostAssignableDeviceTool;

#[async_trait]
impl HyperVTool for GetVmHostAssignableDeviceTool {
    const NAME: &'static str = "hyperv_get_vm_host_assignable_device";
    const DESCRIPTION: &'static str =
        "Retrieves device information assigned to a virtual machine (VM) host.";
    type Input = GetVmHostAssignableDeviceInput;
    type Output = GetVmHostAssignableDeviceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHostAssignableDevice".to_string()];

        if let Some(instance_path) = &input.instance_path {
            if instance_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Instance path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-InstancePath '{}'",
                escape_ps_string(instance_path)
            ));
        }

        if let Some(location_path) = &input.location_path {
            if location_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Location path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-LocationPath '{}'",
                escape_ps_string(location_path)
            ));
        }

        if let Some(resource_pool_name) = &input.resource_pool_name {
            if resource_pool_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Resource pool name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ResourcePoolName '{}'",
                escape_ps_string(resource_pool_name)
            ));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             InstanceId, Name, FriendlyName, LocationPath, ClassName, Vendor, DeviceID, \
             Description, ComputerName, IsGPU, IsSriov, IsDiscrete, IsDda, IsVirtualFunction, \
             IsShareable, IsDismounted, IsNetAdapter, IsReadyForDda, IsInUse | \
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

        let devices = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(devices.len());
        for device in devices {
            output.push(VmHostAssignableDeviceInfo {
                instance_id: device["InstanceId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                name: device["Name"].as_str().unwrap_or_default().to_string(),
                friendly_name: device["FriendlyName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                location_path: device["LocationPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                class_name: device["ClassName"].as_str().unwrap_or_default().to_string(),
                vendor: device["Vendor"].as_str().unwrap_or_default().to_string(),
                device_id: device["DeviceID"].as_str().unwrap_or_default().to_string(),
                description: device["Description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: device["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_gpu: device["IsGPU"].as_bool().unwrap_or_default(),
                is_sriov: device["IsSriov"].as_bool().unwrap_or_default(),
                is_discrete: device["IsDiscrete"].as_bool().unwrap_or_default(),
                is_dda: device["IsDda"].as_bool().unwrap_or_default(),
                is_virtual_function: device["IsVirtualFunction"].as_bool().unwrap_or_default(),
                is_shareable: device["IsShareable"].as_bool().unwrap_or_default(),
                is_dismounted: device["IsDismounted"].as_bool().unwrap_or_default(),
                is_net_adapter: device["IsNetAdapter"].as_bool().unwrap_or_default(),
                is_ready_for_dda: device["IsReadyForDda"].as_bool().unwrap_or_default(),
                is_in_use: device["IsInUse"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmHostAssignableDeviceOutput { devices: output })
    }
}

register_tool!(GetVmHostAssignableDeviceTool);
