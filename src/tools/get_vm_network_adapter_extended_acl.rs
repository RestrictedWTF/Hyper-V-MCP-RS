use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterExtendedAclInput {
    /// Name of the virtual machine whose extended ACLs are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to retrieve ACLs for. If omitted, all matching adapters are returned.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve extended ACLs for network adapters of the management operating system.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterExtendedAclInfo {
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    #[serde(rename = "adapterName")]
    pub adapter_name: Option<String>,
    pub direction: String,
    pub action: String,
    #[serde(rename = "localIpAddress")]
    pub local_ip_address: Option<String>,
    #[serde(rename = "remoteIpAddress")]
    pub remote_ip_address: Option<String>,
    #[serde(rename = "localPort")]
    pub local_port: Option<String>,
    #[serde(rename = "remotePort")]
    pub remote_port: Option<String>,
    pub protocol: Option<String>,
    pub weight: i32,
    pub stateful: bool,
    #[serde(rename = "idleSessionTimeout")]
    pub idle_session_timeout: i32,
    #[serde(rename = "isolationId")]
    pub isolation_id: i32,
    #[serde(rename = "isTemplate")]
    pub is_template: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmNetworkAdapterExtendedAclOutput {
    pub acls: Vec<VmNetworkAdapterExtendedAclInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterExtendedAclTool;

#[async_trait]
impl HyperVTool for GetVmNetworkAdapterExtendedAclTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter_extended_acl";
    const DESCRIPTION: &'static str =
        "Gets extended ACLs configured for a virtual network adapter.";
    type Input = GetVmNetworkAdapterExtendedAclInput;
    type Output = GetVmNetworkAdapterExtendedAclOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.is_none() && !input.management_os {
            return Err(ToolError::InvalidInput(
                "At least one of vm_name or management_os must be specified".to_string(),
            ));
        }

        let mut args = vec!["Get-VMNetworkAdapterExtendedAcl".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(adapter_name) = &input.vm_network_adapter_name {
            if adapter_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(adapter_name)
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
             @{{N='VMName';E={{$_.ParentAdapter.VMName}}}}, \
             @{{N='AdapterName';E={{$_.ParentAdapter.Name}}}}, \
             @{{N='Direction';E={{$_.Direction.ToString()}}}}, \
             @{{N='Action';E={{$_.Action.ToString()}}}}, \
             LocalIPAddress, RemoteIPAddress, LocalPort, RemotePort, Protocol, Weight, Stateful, \
             IdleSessionTimeout, IsolationID, IsTemplate, ComputerName, IsDeleted | \
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

        let acls = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(acls.len());
        for acl in acls {
            output.push(VmNetworkAdapterExtendedAclInfo {
                vm_name: acl["VMName"].as_str().map(String::from),
                adapter_name: acl["AdapterName"].as_str().map(String::from),
                direction: acl["Direction"].as_str().unwrap_or_default().to_string(),
                action: acl["Action"].as_str().unwrap_or_default().to_string(),
                local_ip_address: acl["LocalIPAddress"].as_str().map(String::from),
                remote_ip_address: acl["RemoteIPAddress"].as_str().map(String::from),
                local_port: acl["LocalPort"].as_str().map(String::from),
                remote_port: acl["RemotePort"].as_str().map(String::from),
                protocol: acl["Protocol"].as_str().map(String::from),
                weight: acl["Weight"].as_i64().unwrap_or_default() as i32,
                stateful: acl["Stateful"].as_bool().unwrap_or_default(),
                idle_session_timeout: acl["IdleSessionTimeout"].as_i64().unwrap_or_default() as i32,
                isolation_id: acl["IsolationID"].as_i64().unwrap_or_default() as i32,
                is_template: acl["IsTemplate"].as_bool().unwrap_or_default(),
                computer_name: acl["ComputerName"].as_str().unwrap_or_default().to_string(),
                is_deleted: acl["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmNetworkAdapterExtendedAclOutput { acls: output })
    }
}

register_tool!(GetVmNetworkAdapterExtendedAclTool);
