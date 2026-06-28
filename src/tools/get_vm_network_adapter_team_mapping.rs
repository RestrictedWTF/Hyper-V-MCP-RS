use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterTeamMappingInput {
    /// Name of the virtual machine whose team mappings are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to retrieve. If omitted, all matching adapters are returned.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve team mappings of the management operating system.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
    /// Name of the virtual switch whose team mappings are to be retrieved.
    #[serde(default, rename = "switchName")]
    pub switch_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterTeamMappingInfo {
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    #[serde(rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    #[serde(rename = "netAdapterName")]
    pub net_adapter_name: String,
    #[serde(rename = "netAdapterMacAddress")]
    pub net_adapter_mac_address: String,
    #[serde(rename = "macAddress")]
    pub mac_address: String,
    #[serde(rename = "switchName")]
    pub switch_name: Option<String>,
    #[serde(rename = "parentAdapter")]
    pub parent_adapter: String,
    #[serde(rename = "sourceId")]
    pub source_id: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmNetworkAdapterTeamMappingOutput {
    pub mappings: Vec<VmNetworkAdapterTeamMappingInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterTeamMappingTool;

#[async_trait]
impl HyperVTool for GetVmNetworkAdapterTeamMappingTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter_team_mapping";
    const DESCRIPTION: &'static str =
        "Gets the team mapping settings configured on a virtual network adapter.";
    type Input = GetVmNetworkAdapterTeamMappingInput;
    type Output = GetVmNetworkAdapterTeamMappingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMNetworkAdapterTeamMapping".to_string()];

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
                    "Network adapter name must not be empty".to_string(),
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
        if let Some(switch_name) = &input.switch_name {
            if switch_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Switch name must not be empty".to_string(),
                ));
            }
            args.push(format!("-SwitchName '{}'", escape_ps_string(switch_name)));
        }

        let ps = format!(
            "{} | Select-Object \
             VMName, VMNetworkAdapterName, NetAdapterName, NetAdapterMacAddress, MacAddress, \
             SwitchName, \
             @{{N='ParentAdapter';E={{$_.ParentAdapter.ToString()}}}}, \
             @{{N='SourceId';E={{$_.SourceId.ToString()}}}}, \
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

        let mappings = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(mappings.len());
        for mapping in mappings {
            output.push(VmNetworkAdapterTeamMappingInfo {
                vm_name: mapping["VMName"].as_str().map(String::from),
                vm_network_adapter_name: mapping["VMNetworkAdapterName"].as_str().map(String::from),
                net_adapter_name: mapping["NetAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                net_adapter_mac_address: mapping["NetAdapterMacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                mac_address: mapping["MacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                switch_name: mapping["SwitchName"].as_str().map(String::from),
                parent_adapter: mapping["ParentAdapter"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                source_id: mapping["SourceId"].as_str().unwrap_or_default().to_string(),
                computer_name: mapping["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmNetworkAdapterTeamMappingOutput { mappings: output })
    }
}

register_tool!(GetVmNetworkAdapterTeamMappingTool);
