use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterInput {
    /// Name of the virtual machine whose network adapters are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the network adapter to retrieve. If omitted, all matching adapters are returned.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve network adapters of the management operating system.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
    /// Retrieve all virtual network adapters in the system, including the management OS.
    #[serde(default)]
    pub all: bool,
    /// Return only legacy network adapters. Only valid with vm_name.
    #[serde(default, rename = "isLegacy")]
    pub is_legacy: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterInfo {
    pub name: String,
    #[serde(rename = "adapterId")]
    pub adapter_id: String,
    pub id: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    #[serde(rename = "vmId")]
    pub vm_id: Option<String>,
    #[serde(rename = "switchName")]
    pub switch_name: Option<String>,
    #[serde(rename = "switchId")]
    pub switch_id: Option<String>,
    #[serde(rename = "macAddress")]
    pub mac_address: String,
    #[serde(rename = "isManagementOs")]
    pub is_management_os: bool,
    #[serde(rename = "isLegacy")]
    pub is_legacy: Option<bool>,
    pub status: Vec<String>,
    #[serde(rename = "ipAddresses")]
    pub ip_addresses: Vec<String>,
    #[serde(rename = "vmSnapshotId")]
    pub vm_snapshot_id: Option<String>,
    #[serde(rename = "vmSnapshotName")]
    pub vm_snapshot_name: Option<String>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmNetworkAdapterOutput {
    pub adapters: Vec<VmNetworkAdapterInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterTool;

fn strings_from(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect(),
        serde_json::Value::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

#[async_trait]
impl HyperVTool for GetVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter";
    const DESCRIPTION: &'static str = "Gets the virtual network adapters of a virtual machine, snapshot, management operating system, or of a virtual machine and management operating system.";
    type Input = GetVmNetworkAdapterInput;
    type Output = GetVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.is_none() && !input.management_os && !input.all {
            return Err(ToolError::InvalidInput(
                "At least one of vm_name, management_os, or all must be specified".to_string(),
            ));
        }

        if input.management_os && input.all {
            return Err(ToolError::InvalidInput(
                "Cannot specify both management_os and all".to_string(),
            ));
        }

        if input.vm_name.is_some() && (input.management_os || input.all) {
            return Err(ToolError::InvalidInput(
                "vm_name cannot be combined with management_os or all".to_string(),
            ));
        }

        if input.vm_name.is_none() && input.is_legacy.is_some() {
            return Err(ToolError::InvalidInput(
                "is_legacy requires vm_name".to_string(),
            ));
        }

        let mut args = vec!["Get-VMNetworkAdapter".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if input.management_os {
            args.push("-ManagementOS".to_string());
        }
        if input.all {
            args.push("-All".to_string());
        }
        if let Some(is_legacy) = input.is_legacy {
            args.push(format!("-IsLegacy ${}", is_legacy));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, AdapterId, Id, DeviceId, VMName, VMId, SwitchName, SwitchId, MacAddress, \
             IsManagementOs, IsLegacy, \
             @{{N='Status';E={{@($_.Status | ForEach-Object {{ $_.ToString() }})}}}}, \
             IPAddresses, VMSnapshotId, VMSnapshotName, ComputerName | \
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
                adapter_id: adapter["AdapterId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                id: adapter["Id"].as_str().unwrap_or_default().to_string(),
                device_id: adapter["DeviceId"].as_str().unwrap_or_default().to_string(),
                vm_name: adapter["VMName"].as_str().map(String::from),
                vm_id: adapter["VMId"].as_str().map(String::from),
                switch_name: adapter["SwitchName"].as_str().map(String::from),
                switch_id: adapter["SwitchId"].as_str().map(String::from),
                mac_address: adapter["MacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_management_os: adapter["IsManagementOs"].as_bool().unwrap_or_default(),
                is_legacy: adapter["IsLegacy"].as_bool(),
                status: strings_from(&adapter["Status"]),
                ip_addresses: strings_from(&adapter["IPAddresses"]),
                vm_snapshot_id: adapter["VMSnapshotId"].as_str().map(String::from),
                vm_snapshot_name: adapter["VMSnapshotName"].as_str().map(String::from),
                computer_name: adapter["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmNetworkAdapterOutput { adapters: output })
    }
}

register_tool!(GetVmNetworkAdapterTool);
