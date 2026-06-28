use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmNetworkAdapterRoutingDomainMappingInput {
    /// Name of the virtual machine whose network adapter routing domain mapping is to be set.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter on which to set the routing domain mapping.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Configure the adapter in the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// ID of the routing domain. Either routing_domain_id or routing_domain_name is required.
    #[serde(default, rename = "routingDomainId")]
    pub routing_domain_id: Option<String>,
    /// Name of the routing domain. Either routing_domain_id or routing_domain_name is required.
    #[serde(default, rename = "routingDomainName")]
    pub routing_domain_name: Option<String>,
    /// New name to assign to the routing domain.
    #[serde(default, rename = "newRoutingDomainName")]
    pub new_routing_domain_name: Option<String>,
    /// IDs of virtual subnets to set on the routing domain.
    #[serde(default, rename = "isolationId")]
    pub isolation_id: Vec<i32>,
    /// Names of virtual subnets to set on the routing domain.
    #[serde(default, rename = "isolationName")]
    pub isolation_name: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterRoutingDomainSettingInfo {
    #[serde(rename = "routingDomainId")]
    pub routing_domain_id: String,
    #[serde(rename = "routingDomainName")]
    pub routing_domain_name: String,
    #[serde(rename = "isolationId")]
    pub isolation_id: Vec<i32>,
    #[serde(rename = "isolationName")]
    pub isolation_name: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmNetworkAdapterRoutingDomainMappingOutput {
    pub mappings: Vec<VmNetworkAdapterRoutingDomainSettingInfo>,
}

#[derive(Default)]
pub struct SetVmNetworkAdapterRoutingDomainMappingTool;

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

fn ints_from(value: &serde_json::Value) -> Vec<i32> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_i64().map(|n| n as i32))
            .collect(),
        serde_json::Value::Number(n) => n.as_i64().map(|n| vec![n as i32]).unwrap_or_default(),
        _ => Vec::new(),
    }
}

#[async_trait]
impl HyperVTool for SetVmNetworkAdapterRoutingDomainMappingTool {
    const NAME: &'static str = "hyperv_set_vm_network_adapter_routing_domain_mapping";
    const DESCRIPTION: &'static str = "Sets virtual subnets on a routing domain.";
    type Input = SetVmNetworkAdapterRoutingDomainMappingInput;
    type Output = SetVmNetworkAdapterRoutingDomainMappingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let management_os = input.management_os == Some(true);

        if !management_os {
            match &input.vm_name {
                Some(vm) if !vm.trim().is_empty() => {}
                _ => {
                    return Err(ToolError::InvalidInput(
                        "VM name must be provided when management_os is not enabled".to_string(),
                    ));
                }
            }
        }

        if input.routing_domain_id.is_none() && input.routing_domain_name.is_none() {
            return Err(ToolError::InvalidInput(
                "At least one of routing_domain_id or routing_domain_name must be provided"
                    .to_string(),
            ));
        }

        if input.new_routing_domain_name.is_none()
            && input.isolation_id.is_empty()
            && input.isolation_name.is_empty()
        {
            return Err(ToolError::InvalidInput(
                "At least one of new_routing_domain_name, isolation_id, or isolation_name must be provided".to_string(),
            ));
        }

        if let Some(id) = &input.routing_domain_id {
            if id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "routing_domain_id must not be empty".to_string(),
                ));
            }
        }

        if let Some(name) = &input.routing_domain_name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "routing_domain_name must not be empty".to_string(),
                ));
            }
        }

        if let Some(new_name) = &input.new_routing_domain_name {
            if new_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "new_routing_domain_name must not be empty".to_string(),
                ));
            }
        }

        let mut args = vec!["Set-VMNetworkAdapterRoutingDomainMapping".to_string()];

        if management_os {
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

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(id) = &input.routing_domain_id {
            args.push(format!("-RoutingDomainID '{}'", escape_ps_string(id)));
        }

        if let Some(name) = &input.routing_domain_name {
            args.push(format!("-RoutingDomainName '{}'", escape_ps_string(name)));
        }

        if let Some(new_name) = &input.new_routing_domain_name {
            args.push(format!(
                "-NewRoutingDomainName '{}'",
                escape_ps_string(new_name)
            ));
        }

        if !input.isolation_id.is_empty() {
            let ids = input
                .isolation_id
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            args.push(format!("-IsolationID {}", ids));
        }

        if !input.isolation_name.is_empty() {
            let names = input
                .isolation_name
                .iter()
                .map(|s| format!("'{}'", escape_ps_string(s)))
                .collect::<Vec<_>>()
                .join(",");
            args.push(format!("-IsolationName {}", names));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             @{{N='RoutingDomainID';E={{$_.RoutingDomainID.ToString()}}}}, \
             RoutingDomainName, IsolationID, IsolationName | \
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
            output.push(VmNetworkAdapterRoutingDomainSettingInfo {
                routing_domain_id: mapping["RoutingDomainID"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                routing_domain_name: mapping["RoutingDomainName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                isolation_id: ints_from(&mapping["IsolationID"]),
                isolation_name: strings_from(&mapping["IsolationName"]),
            });
        }

        Ok(SetVmNetworkAdapterRoutingDomainMappingOutput { mappings: output })
    }
}

register_tool!(SetVmNetworkAdapterRoutingDomainMappingTool);
