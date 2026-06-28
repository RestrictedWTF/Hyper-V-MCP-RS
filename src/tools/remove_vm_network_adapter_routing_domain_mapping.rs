use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterRoutingDomainMappingInput {
    /// Name of the virtual machine whose network adapter has the routing domain mapping.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter that has the routing domain mapping.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Remove the mapping from the management operating system adapter instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
    /// ID of the routing domain to remove.
    #[serde(default, rename = "routingDomainID")]
    pub routing_domain_id: Option<String>,
    /// Name of the routing domain to remove.
    #[serde(default, rename = "routingDomainName")]
    pub routing_domain_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RoutingDomainMappingInfo {
    #[serde(rename = "routingDomainID")]
    pub routing_domain_id: String,
    #[serde(rename = "routingDomainName")]
    pub routing_domain_name: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterRoutingDomainMappingOutput {
    /// Routing domain mappings that were removed.
    pub removed: Vec<RoutingDomainMappingInfo>,
}

#[derive(Default)]
pub struct RemoveVmNetworkAdapterRoutingDomainMappingTool;

#[async_trait]
impl HyperVTool for RemoveVmNetworkAdapterRoutingDomainMappingTool {
    const NAME: &'static str = "hyperv_remove_vm_network_adapter_routing_domain_mapping";
    const DESCRIPTION: &'static str = "Removes a routing domain from a virtual network adapter.";
    type Input = RemoveVmNetworkAdapterRoutingDomainMappingInput;
    type Output = RemoveVmNetworkAdapterRoutingDomainMappingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.management_os && input.vm_name.is_some() {
            return Err(ToolError::InvalidInput(
                "vmName cannot be combined with managementOS".to_string(),
            ));
        }

        if !input.management_os {
            match &input.vm_name {
                Some(vm) if !vm.trim().is_empty() => {}
                _ => {
                    return Err(ToolError::InvalidInput(
                        "vmName must be provided when managementOS is not enabled".to_string(),
                    ));
                }
            }
        }

        if input.routing_domain_id.is_none() && input.routing_domain_name.is_none() {
            return Err(ToolError::InvalidInput(
                "At least one of routingDomainID or routingDomainName must be specified"
                    .to_string(),
            ));
        }

        if let Some(id) = &input.routing_domain_id {
            if id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "routingDomainID must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(name) = &input.routing_domain_name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "routingDomainName must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(adapter) = &input.vm_network_adapter_name {
            if adapter.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vmNetworkAdapterName must not be empty when provided".to_string(),
                ));
            }
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec!["Remove-VMNetworkAdapterRoutingDomainMapping".to_string()];

        if input.management_os {
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

        if let Some(id) = &input.routing_domain_id {
            args.push(format!("-RoutingDomainID '{}'", escape_ps_string(id)));
        }

        if let Some(name) = &input.routing_domain_name {
            args.push(format!("-RoutingDomainName '{}'", escape_ps_string(name)));
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             @{{N='RoutingDomainID';E={{$_.RoutingDomainID.ToString()}}}}, \
             RoutingDomainName, VMName, VMNetworkAdapterName, ComputerName | \
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

        let mut removed = Vec::with_capacity(mappings.len());
        for mapping in mappings {
            removed.push(RoutingDomainMappingInfo {
                routing_domain_id: mapping["RoutingDomainID"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                routing_domain_name: mapping["RoutingDomainName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_name: mapping["VMName"].as_str().unwrap_or_default().to_string(),
                vm_network_adapter_name: mapping["VMNetworkAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: mapping["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(RemoveVmNetworkAdapterRoutingDomainMappingOutput { removed })
    }
}

register_tool!(RemoveVmNetworkAdapterRoutingDomainMappingTool);
