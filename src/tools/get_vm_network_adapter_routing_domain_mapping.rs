use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterRoutingDomainMappingInput {
    /// Name of the virtual machine whose routing domain mappings are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// ID of the routing domain to retrieve.
    #[serde(default, rename = "routingDomainId")]
    pub routing_domain_id: Option<String>,
    /// Name of the routing domain to retrieve.
    #[serde(default, rename = "routingDomainName")]
    pub routing_domain_name: Option<String>,
    /// Name of the virtual network adapter whose routing domain mappings are to be retrieved.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve routing domain mappings of the management operating system.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterRoutingDomainMappingInfo {
    #[serde(rename = "routingDomainId")]
    pub routing_domain_id: String,
    #[serde(rename = "routingDomainName")]
    pub routing_domain_name: String,
    #[serde(rename = "isolationIds")]
    pub isolation_ids: Vec<String>,
    #[serde(rename = "isolationNames")]
    pub isolation_names: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmNetworkAdapterRoutingDomainMappingOutput {
    pub mappings: Vec<VmNetworkAdapterRoutingDomainMappingInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterRoutingDomainMappingTool;

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
impl HyperVTool for GetVmNetworkAdapterRoutingDomainMappingTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter_routing_domain_mapping";
    const DESCRIPTION: &'static str = "Gets members of a routing domain.";
    type Input = GetVmNetworkAdapterRoutingDomainMappingInput;
    type Output = GetVmNetworkAdapterRoutingDomainMappingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.management_os && input.vm_name.is_some() {
            return Err(ToolError::InvalidInput(
                "Cannot specify both management_os and vm_name".to_string(),
            ));
        }

        let mut args = vec!["Get-VMNetworkAdapterRoutingDomainMapping".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(routing_domain_id) = &input.routing_domain_id {
            if routing_domain_id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Routing domain ID must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-RoutingDomainID '{}'",
                escape_ps_string(routing_domain_id)
            ));
        }
        if let Some(routing_domain_name) = &input.routing_domain_name {
            if routing_domain_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Routing domain name must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-RoutingDomainName '{}'",
                escape_ps_string(routing_domain_name)
            ));
        }
        if let Some(vm_network_adapter_name) = &input.vm_network_adapter_name {
            if vm_network_adapter_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM network adapter name must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(vm_network_adapter_name)
            ));
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

        let ps = format!(
            "{} | Select-Object \
             RoutingDomainId, RoutingDomainName, \
             @{{N='IsolationId';E={{@($_.IsolationId | ForEach-Object {{ $_.ToString() }})}}}}, \
             @{{N='IsolationName';E={{@($_.IsolationName | ForEach-Object {{ $_.ToString() }})}}}} | \
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
            output.push(VmNetworkAdapterRoutingDomainMappingInfo {
                routing_domain_id: mapping["RoutingDomainId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                routing_domain_name: mapping["RoutingDomainName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                isolation_ids: strings_from(&mapping["IsolationId"]),
                isolation_names: strings_from(&mapping["IsolationName"]),
            });
        }

        Ok(GetVmNetworkAdapterRoutingDomainMappingOutput { mappings: output })
    }
}

register_tool!(GetVmNetworkAdapterRoutingDomainMappingTool);
