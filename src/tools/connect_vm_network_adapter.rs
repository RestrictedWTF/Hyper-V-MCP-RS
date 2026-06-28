use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ConnectVmNetworkAdapterInput {
    /// Name of the virtual machine whose network adapter is to be connected.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to connect. If omitted, all adapters for the VM are connected.
    #[serde(default)]
    pub name: Option<String>,
    /// Name of the virtual switch to connect the adapter to.
    #[serde(rename = "switchName")]
    pub switch_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Connect the adapter in the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterInfo {
    pub name: String,
    #[serde(rename = "adapterId")]
    pub adapter_id: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    #[serde(rename = "vmId")]
    pub vm_id: Option<String>,
    #[serde(rename = "switchName")]
    pub switch_name: String,
    #[serde(rename = "switchId")]
    pub switch_id: Option<String>,
    #[serde(rename = "macAddress")]
    pub mac_address: String,
    #[serde(rename = "isManagementOs")]
    pub is_management_os: bool,
    #[serde(rename = "isLegacy")]
    pub is_legacy: bool,
    pub status: Vec<String>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ConnectVmNetworkAdapterOutput {
    pub adapters: Vec<VmNetworkAdapterInfo>,
}

#[derive(Default)]
pub struct ConnectVmNetworkAdapterTool;

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
impl HyperVTool for ConnectVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_connect_vm_network_adapter";
    const DESCRIPTION: &'static str = "Connects a virtual network adapter to a virtual switch.";
    type Input = ConnectVmNetworkAdapterInput;
    type Output = ConnectVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.switch_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Switch name must not be empty".to_string(),
            ));
        }

        if !input.management_os {
            match &input.vm_name {
                Some(vm) if !vm.trim().is_empty() => {}
                _ => {
                    return Err(ToolError::InvalidInput(
                        "VM name must be provided when management_os is not enabled".to_string(),
                    ));
                }
            }
        }

        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec!["Connect-VMNetworkAdapter".to_string()];

        if input.management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        if let Some(name) = &input.name {
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }

        args.push(format!(
            "-SwitchName '{}'",
            escape_ps_string(&input.switch_name)
        ));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, AdapterId, Id, VMName, VMId, SwitchName, SwitchId, MacAddress, \
             IsManagementOs, IsLegacy, \
             @{{N='Status';E={{@($_.Status | ForEach-Object {{ $_.ToString() }})}}}}, \
             ComputerName | \
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
                vm_name: adapter["VMName"].as_str().map(String::from),
                vm_id: adapter["VMId"].as_str().map(String::from),
                switch_name: adapter["SwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                switch_id: adapter["SwitchId"].as_str().map(String::from),
                mac_address: adapter["MacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_management_os: adapter["IsManagementOs"].as_bool().unwrap_or_default(),
                is_legacy: adapter["IsLegacy"].as_bool().unwrap_or_default(),
                status: strings_from(&adapter["Status"]),
                computer_name: adapter["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(ConnectVmNetworkAdapterOutput { adapters: output })
    }
}

register_tool!(ConnectVmNetworkAdapterTool);
