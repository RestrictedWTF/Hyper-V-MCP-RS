use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterInput {
    /// Name of the virtual machine whose network adapter(s) should be removed.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the network adapter to remove. If omitted, all adapters for the VM are removed.
    #[serde(default, rename = "adapterName")]
    pub adapter_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedNetworkAdapterInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    pub mac_address: String,
    #[serde(rename = "switchName")]
    pub switch_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterOutput {
    /// Network adapters that were removed.
    pub removed: Vec<RemovedNetworkAdapterInfo>,
}

#[derive(Default)]
pub struct RemoveVmNetworkAdapterTool;

#[async_trait]
impl HyperVTool for RemoveVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_remove_vm_network_adapter";
    const DESCRIPTION: &'static str =
        "Removes one or more virtual network adapters from a virtual machine.";
    type Input = RemoveVmNetworkAdapterInput;
    type Output = RemoveVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMNetworkAdapter".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));

        if let Some(adapter) = &input.adapter_name {
            if adapter.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "adapterName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(adapter)));
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
            "{} | Select-Object Name, Id, VMName, MacAddress, SwitchName | ConvertTo-Json -Compress -Depth 3",
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

        let mut removed = Vec::with_capacity(adapters.len());
        for adapter in adapters {
            removed.push(RemovedNetworkAdapterInfo {
                name: adapter["Name"].as_str().unwrap_or_default().to_string(),
                id: adapter["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: adapter["VMName"].as_str().unwrap_or_default().to_string(),
                mac_address: adapter["MacAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                switch_name: adapter["SwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(RemoveVmNetworkAdapterOutput { removed })
    }
}

register_tool!(RemoveVmNetworkAdapterTool);
