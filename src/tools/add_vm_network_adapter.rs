use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmNetworkAdapterInput {
    /// Name of the virtual machine to which the network adapter is added.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name to assign to the new network adapter.
    #[serde(default)]
    pub name: Option<String>,
    /// Name of the virtual switch to connect the adapter to.
    #[serde(default, rename = "switchName")]
    pub switch_name: Option<String>,
    /// Specifies whether to add a legacy network adapter.
    #[serde(default, rename = "isLegacy")]
    pub is_legacy: Option<bool>,
    /// Specifies whether to use a dynamic MAC address.
    #[serde(default, rename = "dynamicMacAddress")]
    pub dynamic_mac_address: Option<bool>,
    /// Specifies whether to enable NUMA-aware placement.
    #[serde(default, rename = "numaAwarePlacement")]
    pub numa_aware_placement: Option<bool>,
    /// Static MAC address to assign to the adapter.
    #[serde(default, rename = "staticMacAddress")]
    pub static_mac_address: Option<String>,
    /// Name of the resource pool to assign the adapter to.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
    /// Device naming setting: On or Off.
    #[serde(default, rename = "deviceNaming")]
    pub device_naming: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "switchName")]
    pub switch_name: String,
    #[serde(rename = "macAddress")]
    pub mac_address: String,
    #[serde(rename = "isLegacy")]
    pub is_legacy: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmNetworkAdapterOutput {
    pub adapters: Vec<VmNetworkAdapterInfo>,
}

#[derive(Default)]
pub struct AddVmNetworkAdapterTool;

#[async_trait]
impl HyperVTool for AddVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_add_vm_network_adapter";
    const DESCRIPTION: &'static str = "Adds a virtual network adapter to a virtual machine.";
    type Input = AddVmNetworkAdapterInput;
    type Output = AddVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Add-VMNetworkAdapter -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(switch) = &input.switch_name {
            if switch.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Switch name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-SwitchName '{}'", escape_ps_string(switch)));
        }
        if let Some(is_legacy) = input.is_legacy {
            args.push(format!("-IsLegacy ${}", is_legacy));
        }
        if let Some(dynamic) = input.dynamic_mac_address {
            args.push(format!("-DynamicMacAddress:${}", dynamic));
        }
        if let Some(numa) = input.numa_aware_placement {
            args.push(format!("-NumaAwarePlacement ${}", numa));
        }
        if let Some(mac) = &input.static_mac_address {
            if mac.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Static MAC address must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-StaticMacAddress '{}'", escape_ps_string(mac)));
        }
        if let Some(pool) = &input.resource_pool_name {
            if pool.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Resource pool name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ResourcePoolName '{}'", escape_ps_string(pool)));
        }
        if let Some(naming) = &input.device_naming {
            if naming.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Device naming must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-DeviceNaming '{}'", escape_ps_string(naming)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }


        let ps = format!(
            "{} | Select-Object Name, Id, VMName, VMId, SwitchName, MacAddress, IsLegacy | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let adapters = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(adapters.len());
        for adapter in adapters {
            output.push(VmNetworkAdapterInfo {
                name: adapter["Name"].as_str().unwrap_or_default().to_string(),
                id: adapter["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: adapter["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: adapter["VMId"].as_str().unwrap_or_default().to_string(),
                switch_name: adapter["SwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                mac_address: adapter["MacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_legacy: adapter["IsLegacy"].as_bool().unwrap_or_default(),
            });
        }

        Ok(AddVmNetworkAdapterOutput { adapters: output })
    }
}

register_tool!(AddVmNetworkAdapterTool);
