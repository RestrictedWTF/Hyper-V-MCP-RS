use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterExtendedAclInput {
    /// Name of the virtual machine whose extended ACL should be removed.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter. If omitted, applies to all adapters of the VM.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Weight of the extended ACL entry to remove.
    pub weight: i32,
    /// Direction of the ACL to remove: Inbound or Outbound.
    pub direction: String,
    /// Target the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterExtendedAclInfo {
    pub weight: i32,
    pub direction: String,
    pub action: String,
    pub protocol: Option<String>,
    #[serde(rename = "localIPAddress")]
    pub local_ip_address: Option<String>,
    #[serde(rename = "remoteIPAddress")]
    pub remote_ip_address: Option<String>,
    #[serde(rename = "localPort")]
    pub local_port: Option<String>,
    #[serde(rename = "remotePort")]
    pub remote_port: Option<String>,
    pub stateful: Option<bool>,
    #[serde(rename = "idleSessionTimeout")]
    pub idle_session_timeout: Option<i32>,
    #[serde(rename = "isolationID")]
    pub isolation_id: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterExtendedAclOutput {
    /// Extended ACLs that were removed.
    pub removed: Vec<VmNetworkAdapterExtendedAclInfo>,
}

#[derive(Default)]
pub struct RemoveVmNetworkAdapterExtendedAclTool;

#[async_trait]
impl HyperVTool for RemoveVmNetworkAdapterExtendedAclTool {
    const NAME: &'static str = "hyperv_remove_vm_network_adapter_extended_acl";
    const DESCRIPTION: &'static str = "Removes an extended ACL for a virtual network adapter.";
    type Input = RemoveVmNetworkAdapterExtendedAclInput;
    type Output = RemoveVmNetworkAdapterExtendedAclOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.is_none() && !input.management_os {
            return Err(ToolError::InvalidInput(
                "Either vmName or managementOS must be specified".to_string(),
            ));
        }

        if input.vm_name.is_some() && input.management_os {
            return Err(ToolError::InvalidInput(
                "Cannot specify both vmName and managementOS".to_string(),
            ));
        }

        let direction = input.direction.trim();
        if direction.is_empty() {
            return Err(ToolError::InvalidInput(
                "direction must not be empty".to_string(),
            ));
        }
        if !direction.eq_ignore_ascii_case("Inbound") && !direction.eq_ignore_ascii_case("Outbound")
        {
            return Err(ToolError::InvalidInput(
                "direction must be Inbound or Outbound".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMNetworkAdapterExtendedAcl".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vmName must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }

        if input.management_os {
            args.push("-ManagementOS".to_string());
        }

        if let Some(adapter_name) = &input.vm_network_adapter_name {
            if adapter_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vmNetworkAdapterName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(adapter_name)
            ));
        }

        args.push(format!("-Weight {}", input.weight));
        args.push(format!("-Direction '{}'", escape_ps_string(direction)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             Weight, \
             @{{N='Direction';E={{$_.Direction.ToString()}}}}, \
             @{{N='Action';E={{$_.Action.ToString()}}}}, \
             Protocol, LocalIPAddress, RemoteIPAddress, LocalPort, RemotePort, \
             Stateful, IdleSessionTimeout, IsolationID | \
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

        let mut removed = Vec::with_capacity(acls.len());
        for acl in acls {
            removed.push(VmNetworkAdapterExtendedAclInfo {
                weight: acl["Weight"].as_i64().unwrap_or_default() as i32,
                direction: acl["Direction"].as_str().unwrap_or_default().to_string(),
                action: acl["Action"].as_str().unwrap_or_default().to_string(),
                protocol: acl["Protocol"].as_str().map(String::from),
                local_ip_address: acl["LocalIPAddress"].as_str().map(String::from),
                remote_ip_address: acl["RemoteIPAddress"].as_str().map(String::from),
                local_port: acl["LocalPort"].as_str().map(String::from),
                remote_port: acl["RemotePort"].as_str().map(String::from),
                stateful: acl["Stateful"].as_bool(),
                idle_session_timeout: acl["IdleSessionTimeout"].as_i64().map(|v| v as i32),
                isolation_id: acl["IsolationID"].as_i64().map(|v| v as i32),
            });
        }

        Ok(RemoveVmNetworkAdapterExtendedAclOutput { removed })
    }
}

register_tool!(RemoveVmNetworkAdapterExtendedAclTool);
