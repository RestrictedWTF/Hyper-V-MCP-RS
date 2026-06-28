use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterFailoverConfigSetInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "networkAdapterName")]
    pub network_adapter_name: String,
    #[serde(rename = "ipAddresses")]
    pub ip_addresses: Vec<String>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmNetworkAdapterFailoverConfigurationInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the network adapter.
    #[serde(rename = "networkAdapterName")]
    pub network_adapter_name: String,
    /// IP addresses for failover.
    #[serde(rename = "ipAddresses")]
    pub ip_addresses: Vec<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmNetworkAdapterFailoverConfigurationOutput {
    pub configs: Vec<VmNetworkAdapterFailoverConfigSetInfo>,
}

#[derive(Default)]
pub struct SetVmNetworkAdapterFailoverConfigurationTool;

#[async_trait]
impl HyperVTool for SetVmNetworkAdapterFailoverConfigurationTool {
    const NAME: &'static str = "hyperv_set_vm_network_adapter_failover_configuration";
    const DESCRIPTION: &'static str = "Configures the IP address of a virtual network adapter to be used when a virtual machine fails over.";
    type Input = SetVmNetworkAdapterFailoverConfigurationInput;
    type Output = SetVmNetworkAdapterFailoverConfigurationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMNetworkAdapterFailoverConfiguration".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_name must not be empty".to_string(),
            ));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if input.network_adapter_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "network_adapter_name must not be empty".to_string(),
            ));
        }
        args.push(format!(
            "-NetworkAdapterName '{}'",
            escape_ps_string(&input.network_adapter_name)
        ));
        if !input.ip_addresses.is_empty() {
            let escaped: Vec<String> = input
                .ip_addresses
                .iter()
                .map(|ip| format!("'{}'", escape_ps_string(ip)))
                .collect();
            args.push(format!("-IPAddress @({})", escaped.join(",")));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = format!("{} | Select-Object VMName, NetworkAdapterName, IPAddresses, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            let ip_addresses = match &item["IPAddresses"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                _ => Vec::new(),
            };
            output.push(VmNetworkAdapterFailoverConfigSetInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                network_adapter_name: item["NetworkAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                ip_addresses,
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmNetworkAdapterFailoverConfigurationOutput { configs: output })
    }
}

register_tool!(SetVmNetworkAdapterFailoverConfigurationTool);
