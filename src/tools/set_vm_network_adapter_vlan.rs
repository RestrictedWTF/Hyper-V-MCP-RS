use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmNetworkAdapterVlanInput {
    /// Name of the virtual machine whose network adapter VLAN is to be configured.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Configure the adapter in the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Name of the virtual network adapter to configure.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// VLAN operation mode: Access, Untagged, Trunk, Isolated, Community, or Promiscuous.
    #[serde(rename = "operationMode")]
    pub operation_mode: String,
    /// VLAN ID for Access mode.
    #[serde(default, rename = "vlanId")]
    pub vlan_id: Option<i32>,
    /// Native VLAN ID for Trunk mode.
    #[serde(default, rename = "nativeVlanId")]
    pub native_vlan_id: Option<i32>,
    /// Comma-separated or ranged list of allowed VLAN IDs for Trunk mode.
    #[serde(default, rename = "allowedVlanIdList")]
    pub allowed_vlan_id_list: Option<String>,
    /// Primary VLAN ID for Private VLAN modes.
    #[serde(default, rename = "primaryVlanId")]
    pub primary_vlan_id: Option<i32>,
    /// Secondary VLAN ID for Isolated or Community mode.
    #[serde(default, rename = "secondaryVlanId")]
    pub secondary_vlan_id: Option<i32>,
    /// Comma-separated or ranged list of secondary VLAN IDs for Promiscuous mode.
    #[serde(default, rename = "secondaryVlanIdList")]
    pub secondary_vlan_id_list: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterVlanInfo {
    pub vm_name: String,
    pub vm_network_adapter_name: String,
    pub computer_name: String,
    pub operation_mode: String,
    pub access_vlan_id: i32,
    pub native_vlan_id: i32,
    pub allowed_vlan_id_list: String,
    pub primary_vlan_id: i32,
    pub secondary_vlan_id: i32,
    pub secondary_vlan_id_list: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmNetworkAdapterVlanOutput {
    pub adapters: Vec<VmNetworkAdapterVlanInfo>,
}

#[derive(Default)]
pub struct SetVmNetworkAdapterVlanTool;

#[async_trait]
impl HyperVTool for SetVmNetworkAdapterVlanTool {
    const NAME: &'static str = "hyperv_set_vm_network_adapter_vlan";
    const DESCRIPTION: &'static str =
        "Configures the virtual LAN settings for the traffic through a virtual network adapter.";
    type Input = SetVmNetworkAdapterVlanInput;
    type Output = SetVmNetworkAdapterVlanOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
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

        let mode = input.operation_mode.trim();
        if mode.is_empty() {
            return Err(ToolError::InvalidInput(
                "operation_mode must not be empty".to_string(),
            ));
        }

        let normalized_mode = match mode.to_ascii_lowercase().as_str() {
            "access" => "Access",
            "untagged" => "Untagged",
            "trunk" => "Trunk",
            "isolated" => "Isolated",
            "community" => "Community",
            "promiscuous" => "Promiscuous",
            _ => {
                return Err(ToolError::InvalidInput(
                    "operation_mode must be one of: Access, Untagged, Trunk, Isolated, Community, Promiscuous".to_string(),
                ));
            }
        };

        let mut args = vec!["Set-VMNetworkAdapterVlan".to_string()];

        if management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        if let Some(adapter) = &input.vm_network_adapter_name {
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(adapter)
            ));
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        match normalized_mode {
            "Access" => {
                args.push("-Access".to_string());
                match input.vlan_id {
                    Some(id) => args.push(format!("-VlanId {}", id)),
                    None => {
                        return Err(ToolError::InvalidInput(
                            "vlan_id is required for Access mode".to_string(),
                        ));
                    }
                }
            }
            "Untagged" => {
                args.push("-Untagged".to_string());
            }
            "Trunk" => {
                args.push("-Trunk".to_string());
                match input.native_vlan_id {
                    Some(id) => args.push(format!("-NativeVlanId {}", id)),
                    None => {
                        return Err(ToolError::InvalidInput(
                            "native_vlan_id is required for Trunk mode".to_string(),
                        ));
                    }
                }
                match &input.allowed_vlan_id_list {
                    Some(list) if !list.trim().is_empty() => {
                        args.push(format!("-AllowedVlanIdList '{}'", escape_ps_string(list)));
                    }
                    _ => {
                        return Err(ToolError::InvalidInput(
                            "allowed_vlan_id_list is required for Trunk mode".to_string(),
                        ));
                    }
                }
            }
            "Isolated" | "Community" => {
                args.push(format!("-{}", normalized_mode));
                match input.primary_vlan_id {
                    Some(id) => args.push(format!("-PrimaryVlanId {}", id)),
                    None => {
                        return Err(ToolError::InvalidInput(
                            "primary_vlan_id is required for Isolated/Community mode".to_string(),
                        ));
                    }
                }
                match input.secondary_vlan_id {
                    Some(id) => args.push(format!("-SecondaryVlanId {}", id)),
                    None => {
                        return Err(ToolError::InvalidInput(
                            "secondary_vlan_id is required for Isolated/Community mode".to_string(),
                        ));
                    }
                }
            }
            "Promiscuous" => {
                args.push("-Promiscuous".to_string());
                match input.primary_vlan_id {
                    Some(id) => args.push(format!("-PrimaryVlanId {}", id)),
                    None => {
                        return Err(ToolError::InvalidInput(
                            "primary_vlan_id is required for Promiscuous mode".to_string(),
                        ));
                    }
                }
                match &input.secondary_vlan_id_list {
                    Some(list) if !list.trim().is_empty() => {
                        args.push(format!("-SecondaryVlanIdList '{}'", escape_ps_string(list)));
                    }
                    _ => {
                        return Err(ToolError::InvalidInput(
                            "secondary_vlan_id_list is required for Promiscuous mode".to_string(),
                        ));
                    }
                }
            }
            _ => unreachable!(),
        }

        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object VMName, VMNetworkAdapterName, ComputerName, \
             @{{N='OperationMode';E={{$_.OperationMode.ToString()}}}}, \
             AccessVlanId, NativeVlanId, AllowedVlanIdList, PrimaryVlanId, SecondaryVlanId, SecondaryVlanIdList | \
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
            output.push(VmNetworkAdapterVlanInfo {
                vm_name: adapter["VMName"].as_str().unwrap_or_default().to_string(),
                vm_network_adapter_name: adapter["VMNetworkAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: adapter["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                operation_mode: adapter["OperationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                access_vlan_id: adapter["AccessVlanId"].as_i64().unwrap_or_default() as i32,
                native_vlan_id: adapter["NativeVlanId"].as_i64().unwrap_or_default() as i32,
                allowed_vlan_id_list: adapter["AllowedVlanIdList"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                primary_vlan_id: adapter["PrimaryVlanId"].as_i64().unwrap_or_default() as i32,
                secondary_vlan_id: adapter["SecondaryVlanId"].as_i64().unwrap_or_default() as i32,
                secondary_vlan_id_list: adapter["SecondaryVlanIdList"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmNetworkAdapterVlanOutput { adapters: output })
    }
}

register_tool!(SetVmNetworkAdapterVlanTool);
