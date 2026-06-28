use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmNetworkAdapterRoutingDomainMappingInput {
    /// Friendly name of the virtual machine whose network adapter receives the mapping.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Operate on the management (host) operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Name of the virtual network adapter to target.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// GUID of the routing domain to add.
    #[serde(rename = "routingDomainId")]
    pub routing_domain_id: String,
    /// Name of the routing domain to add.
    #[serde(rename = "routingDomainName")]
    pub routing_domain_name: String,
    /// IDs of the virtual subnets to add to the routing domain.
    #[serde(rename = "isolationId")]
    pub isolation_id: Vec<i32>,
    /// Names of the virtual subnets to add to the routing domain.
    #[serde(default, rename = "isolationName")]
    pub isolation_name: Vec<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RoutingDomainMappingInfo {
    #[serde(rename = "routingDomainId")]
    pub routing_domain_id: String,
    #[serde(rename = "routingDomainName")]
    pub routing_domain_name: String,
    #[serde(rename = "isolationId")]
    pub isolation_id: Vec<i32>,
    #[serde(rename = "isolationName")]
    pub isolation_name: Vec<String>,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmNetworkAdapterRoutingDomainMappingOutput {
    pub mappings: Vec<RoutingDomainMappingInfo>,
}

#[derive(Default)]
pub struct AddVmNetworkAdapterRoutingDomainMappingTool;

#[async_trait]
impl HyperVTool for AddVmNetworkAdapterRoutingDomainMappingTool {
    const NAME: &'static str = "hyperv_add_vm_network_adapter_routing_domain_mapping";
    const DESCRIPTION: &'static str =
        "Adds a routing domain and virtual subnets to a virtual network adapter.";
    type Input = AddVmNetworkAdapterRoutingDomainMappingInput;
    type Output = AddVmNetworkAdapterRoutingDomainMappingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let is_management_os = input.management_os.unwrap_or(false);

        if !is_management_os
            && input
                .vm_name
                .as_ref()
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
        {
            return Err(ToolError::InvalidInput(
                "VMName is required unless ManagementOS is true".to_string(),
            ));
        }

        if input.routing_domain_id.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "RoutingDomainID must not be empty".to_string(),
            ));
        }

        if input.routing_domain_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "RoutingDomainName must not be empty".to_string(),
            ));
        }

        if input.isolation_id.is_empty() {
            return Err(ToolError::InvalidInput(
                "At least one IsolationID must be provided".to_string(),
            ));
        }

        let mut args = vec!["Add-VMNetworkAdapterRoutingDomainMapping".to_string()];

        if is_management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm_name) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }

        args.push(format!(
            "-RoutingDomainID '{}'",
            escape_ps_string(&input.routing_domain_id)
        ));
        args.push(format!(
            "-RoutingDomainName '{}'",
            escape_ps_string(&input.routing_domain_name)
        ));

        let isolation_ids = input
            .isolation_id
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(",");
        args.push(format!("-IsolationID {}", isolation_ids));

        if !input.isolation_name.is_empty() {
            let isolation_names = input
                .isolation_name
                .iter()
                .map(|name| format!("'{}'", escape_ps_string(name)))
                .collect::<Vec<_>>()
                .join(",");
            args.push(format!("-IsolationName {}", isolation_names));
        }

        if let Some(adapter_name) = &input.vm_network_adapter_name {
            if !adapter_name.trim().is_empty() {
                args.push(format!(
                    "-VMNetworkAdapterName '{}'",
                    escape_ps_string(adapter_name)
                ));
            }
        }

        if let Some(computer) = &input.computer_name {
            if !computer.trim().is_empty() {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
        }


        let ps = format!(
            "{} | Select-Object \
             @{{N='RoutingDomainId';E={{$_.RoutingDomainId.ToString()}}}}, \
             RoutingDomainName, \
             IsolationId, \
             IsolationName, \
             VMName, \
             VMNetworkAdapterName, \
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
            output.push(RoutingDomainMappingInfo {
                routing_domain_id: mapping["RoutingDomainId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                routing_domain_name: mapping["RoutingDomainName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                isolation_id: parse_i32_array(&mapping["IsolationId"]),
                isolation_name: parse_string_array(&mapping["IsolationName"]),
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

        Ok(AddVmNetworkAdapterRoutingDomainMappingOutput { mappings: output })
    }
}

fn parse_i32_array(value: &serde_json::Value) -> Vec<i32> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_i64().unwrap_or_default() as i32)
            .collect(),
        serde_json::Value::Number(n) => vec![n.as_i64().unwrap_or_default() as i32],
        serde_json::Value::String(s) => s
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_string_array(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect(),
        serde_json::Value::String(s) if !s.is_empty() => vec![s.clone()],
        _ => Vec::new(),
    }
}

register_tool!(AddVmNetworkAdapterRoutingDomainMappingTool);
