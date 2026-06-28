use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterRenameInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "macAddress")]
    pub mac_address: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameVmNetworkAdapterInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Current name of the virtual network adapter.
pub name: String,
    /// New name for the network adapter.
    #[serde(rename = "newName")]
    pub new_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RenameVmNetworkAdapterOutput {
    pub adapters: Vec<VmNetworkAdapterRenameInfo>,
}


#[derive(Default)]
pub struct RenameVmNetworkAdapterTool;

#[async_trait]
impl HyperVTool for RenameVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_rename_vm_network_adapter";
    const DESCRIPTION: &'static str = "Renames a virtual network adapter on a virtual machine or on the management operating system.";
    type Input = RenameVmNetworkAdapterInput;
    type Output = RenameVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Rename-VMNetworkAdapter".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("vm_name must not be empty when provided".to_string()));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if input.new_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("new_name must not be empty".to_string()));
        }
        args.push(format!("-NewName '{}'", escape_ps_string(&input.new_name)));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        args.push("-PassThru".to_string());
        let ps = format!("{} | Select-Object Name, Id, VMName, VMId, MacAddress, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            output.push(VmNetworkAdapterRenameInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                mac_address: item["MacAddress"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(RenameVmNetworkAdapterOutput { adapters: output })

    }
}


register_tool!(RenameVmNetworkAdapterTool);
