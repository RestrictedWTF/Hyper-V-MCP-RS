use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmNetworkAdapterInput {
    /// Name of the virtual machine whose network adapter is to be configured.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to configure.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Configure the adapter in the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Virtual switch to connect the adapter to.
    #[serde(default, rename = "switchName")]
    pub switch_name: Option<String>,
    /// Static MAC address to assign to the adapter.
    #[serde(default, rename = "staticMacAddress")]
    pub static_mac_address: Option<String>,
    /// Enable or disable dynamic MAC address generation.
    #[serde(default, rename = "dynamicMacAddressEnabled")]
    pub dynamic_mac_address_enabled: Option<bool>,
    /// Enable MAC address spoofing. Valid values: On, Off.
    #[serde(default, rename = "macAddressSpoofing")]
    pub mac_address_spoofing: Option<String>,
    /// Allow NIC teaming in the VM. Valid values: On, Off.
    #[serde(default, rename = "allowTeaming")]
    pub allow_teaming: Option<String>,
    /// Enable device naming. Valid values: On, Off.
    #[serde(default, rename = "deviceNaming")]
    pub device_naming: Option<String>,
    /// Enable IPsec task offload. Valid values: Enabled, Disabled.
    #[serde(default, rename = "ipsecTaskOffload")]
    pub ipsec_task_offload: Option<String>,
    /// Enable Virtual Machine Multi-Queue.
    #[serde(default, rename = "vmmqEnabled")]
    pub vmmq_enabled: Option<bool>,
    /// Enable Virtual Receive Side Scaling.
    #[serde(default, rename = "vrssEnabled")]
    pub vrss_enabled: Option<bool>,
    /// Enable Packet Direct.
    #[serde(default, rename = "packetDirectEnabled")]
    pub packet_direct_enabled: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterInfo {
    pub name: String,
    pub is_legacy: bool,
    pub switch_name: String,
    pub mac_address: String,
    pub dynamic_mac_address_enabled: bool,
    pub mac_address_spoofing: String,
    pub allow_teaming: String,
    pub device_naming: String,
    pub ipsec_task_offload: String,
    pub vmmq_enabled: bool,
    pub vrss_enabled: bool,
    pub packet_direct_enabled: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmNetworkAdapterOutput {
    pub adapters: Vec<VmNetworkAdapterInfo>,
}

#[derive(Default)]
pub struct SetVmNetworkAdapterTool;

#[async_trait]
impl HyperVTool for SetVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_set_vm_network_adapter";
    const DESCRIPTION: &'static str =
        "Configures features of the virtual network adapter in a virtual machine or the management operating system.";
    type Input = SetVmNetworkAdapterInput;
    type Output = SetVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Network adapter name must not be empty".to_string(),
            ));
        }

        let management_os = input.management_os == Some(true);
        if !management_os {
            match &input.vm_name {
                Some(vm) if !vm.trim().is_empty() => {}
                _ => {
                    return Err(ToolError::InvalidInput(
                        "VM name must be provided when management_os is not enabled".to_string(),
                    ));
                }
            }
        }

        if input.switch_name.is_none()
            && input.static_mac_address.is_none()
            && input.dynamic_mac_address_enabled.is_none()
            && input.mac_address_spoofing.is_none()
            && input.allow_teaming.is_none()
            && input.device_naming.is_none()
            && input.ipsec_task_offload.is_none()
            && input.vmmq_enabled.is_none()
            && input.vrss_enabled.is_none()
            && input.packet_direct_enabled.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one network adapter setting must be provided".to_string(),
            ));
        }

        let mut args = vec!["Set-VMNetworkAdapter".to_string()];

        if management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(switch) = &input.switch_name {
            args.push(format!("-SwitchName '{}'", escape_ps_string(switch)));
        }
        if let Some(mac) = &input.static_mac_address {
            args.push(format!("-StaticMacAddress '{}'", escape_ps_string(mac)));
        }
        if let Some(enabled) = input.dynamic_mac_address_enabled {
            args.push(format!("-DynamicMacAddressEnabled ${}", enabled));
        }
        if let Some(spoofing) = &input.mac_address_spoofing {
            args.push(format!(
                "-MacAddressSpoofing '{}'",
                escape_ps_string(spoofing)
            ));
        }
        if let Some(teaming) = &input.allow_teaming {
            args.push(format!("-AllowTeaming '{}'", escape_ps_string(teaming)));
        }
        if let Some(naming) = &input.device_naming {
            args.push(format!("-DeviceNaming '{}'", escape_ps_string(naming)));
        }
        if let Some(offload) = &input.ipsec_task_offload {
            args.push(format!("-IPsecTaskOffload '{}'", escape_ps_string(offload)));
        }
        if let Some(enabled) = input.vmmq_enabled {
            args.push(format!("-VmmqEnabled ${}", enabled));
        }
        if let Some(enabled) = input.vrss_enabled {
            args.push(format!("-VrssEnabled ${}", enabled));
        }
        if let Some(enabled) = input.packet_direct_enabled {
            args.push(format!("-PacketDirectEnabled ${}", enabled));
        }

        let ps = format!(
            "{} | Select-Object Name, IsLegacy, SwitchName, MacAddress, \
             DynamicMacAddressEnabled, \
             @{{N='MacAddressSpoofing';E={{$_.MacAddressSpoofing.ToString()}}}}, \
             @{{N='AllowTeaming';E={{$_.AllowTeaming.ToString()}}}}, \
             @{{N='DeviceNaming';E={{$_.DeviceNaming.ToString()}}}}, \
             @{{N='IPsecTaskOffload';E={{$_.IPsecTaskOffload.ToString()}}}}, \
             VmmqEnabled, VrssEnabled, PacketDirectEnabled | \
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

        let adapters = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(adapters.len());
        for adapter in adapters {
            output.push(VmNetworkAdapterInfo {
                name: adapter["Name"].as_str().unwrap_or_default().to_string(),
                is_legacy: adapter["IsLegacy"].as_bool().unwrap_or_default(),
                switch_name: adapter["SwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                mac_address: adapter["MacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                dynamic_mac_address_enabled: adapter["DynamicMacAddressEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
                mac_address_spoofing: adapter["MacAddressSpoofing"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                allow_teaming: adapter["AllowTeaming"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                device_naming: adapter["DeviceNaming"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                ipsec_task_offload: adapter["IPsecTaskOffload"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vmmq_enabled: adapter["VmmqEnabled"].as_bool().unwrap_or_default(),
                vrss_enabled: adapter["VrssEnabled"].as_bool().unwrap_or_default(),
                packet_direct_enabled: adapter["PacketDirectEnabled"].as_bool().unwrap_or_default(),
            });
        }

        Ok(SetVmNetworkAdapterOutput { adapters: output })
    }
}

register_tool!(SetVmNetworkAdapterTool);
