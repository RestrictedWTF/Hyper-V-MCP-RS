use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterVlanInput {
    /// Name of the virtual machine whose adapter VLAN settings are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter whose VLAN settings are to be retrieved.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve VLAN settings for adapters of the management operating system.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterVlanInfo {
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    #[serde(rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: String,
    #[serde(rename = "operationMode")]
    pub operation_mode: String,
    #[serde(rename = "accessVlanId")]
    pub access_vlan_id: Option<u32>,
    #[serde(rename = "nativeVlanId")]
    pub native_vlan_id: Option<u32>,
    #[serde(rename = "allowedVlanIdList")]
    pub allowed_vlan_id_list: Option<String>,
    #[serde(rename = "primaryVlanId")]
    pub primary_vlan_id: Option<u32>,
    #[serde(rename = "secondaryVlanId")]
    pub secondary_vlan_id: Option<u32>,
    #[serde(rename = "secondaryVlanIdList")]
    pub secondary_vlan_id_list: Option<String>,
    #[serde(rename = "isManagementOs")]
    pub is_management_os: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmNetworkAdapterVlanOutput {
    pub vlans: Vec<VmNetworkAdapterVlanInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterVlanTool;

fn opt_u32_from(value: &serde_json::Value) -> Option<u32> {
    value.as_u64().map(|v| v as u32)
}

fn opt_string_from(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

#[async_trait]
impl HyperVTool for GetVmNetworkAdapterVlanTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter_vlan";
    const DESCRIPTION: &'static str =
        "Gets the virtual LAN settings configured on a virtual network adapter.";
    type Input = GetVmNetworkAdapterVlanInput;
    type Output = GetVmNetworkAdapterVlanOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.management_os && input.vm_name.is_some() {
            return Err(ToolError::InvalidInput(
                "management_os cannot be combined with vm_name".to_string(),
            ));
        }

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
        }
        if let Some(adapter_name) = &input.vm_network_adapter_name {
            if adapter_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty".to_string(),
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
        }

        let mut args = vec!["Get-VMNetworkAdapterVlan".to_string()];

        if input.management_os {
            args.push("-ManagementOS".to_string());
        }
        if let Some(vm_name) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(adapter_name) = &input.vm_network_adapter_name {
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(adapter_name)
            ));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             VMName, VMNetworkAdapterName, \
             @{{N='OperationMode';E={{$_.OperationMode.ToString()}}}}, \
             AccessVlanId, NativeVlanId, AllowedVlanIdList, \
             PrimaryVlanId, SecondaryVlanId, SecondaryVlanIdList, \
             IsManagementOs, ComputerName | \
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

        let vlans = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(vlans.len());
        for vlan in vlans {
            output.push(VmNetworkAdapterVlanInfo {
                vm_name: vlan["VMName"].as_str().map(String::from),
                vm_network_adapter_name: vlan["VMNetworkAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                operation_mode: vlan["OperationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                access_vlan_id: opt_u32_from(&vlan["AccessVlanId"]),
                native_vlan_id: opt_u32_from(&vlan["NativeVlanId"]),
                allowed_vlan_id_list: opt_string_from(&vlan["AllowedVlanIdList"]),
                primary_vlan_id: opt_u32_from(&vlan["PrimaryVlanId"]),
                secondary_vlan_id: opt_u32_from(&vlan["SecondaryVlanId"]),
                secondary_vlan_id_list: opt_string_from(&vlan["SecondaryVlanIdList"]),
                is_management_os: vlan["IsManagementOs"].as_bool().unwrap_or_default(),
                computer_name: vlan["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmNetworkAdapterVlanOutput { vlans: output })
    }
}

register_tool!(GetVmNetworkAdapterVlanTool);
