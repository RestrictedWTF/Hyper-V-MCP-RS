use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmNetworkAdapterExtendedAclInput {
    /// Name of the virtual machine to which the extended ACL is added.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Action for the ACL: Allow or Deny.
    pub action: String,
    /// Direction of traffic: Inbound or Outbound.
    pub direction: String,
    /// Weight of the ACL entry. Higher values apply first.
    pub weight: i32,
    /// Local IP address for the ACL.
    #[serde(default, rename = "localIpAddress")]
    pub local_ip_address: Option<String>,
    /// Remote IP address for the ACL.
    #[serde(default, rename = "remoteIpAddress")]
    pub remote_ip_address: Option<String>,
    /// Local port or port range for the ACL.
    #[serde(default, rename = "localPort")]
    pub local_port: Option<String>,
    /// Remote port or port range for the ACL.
    #[serde(default, rename = "remotePort")]
    pub remote_port: Option<String>,
    /// Protocol for the ACL, such as TCP, UDP, or an IP protocol number.
    #[serde(default)]
    pub protocol: Option<String>,
    /// Whether the ACL is stateful.
    #[serde(default, rename = "stateful")]
    pub stateful: Option<bool>,
    /// Idle session timeout in seconds for stateful ACLs.
    #[serde(default, rename = "idleSessionTimeout")]
    pub idle_session_timeout: Option<i32>,
    /// Isolation ID of a virtual subnet for the ACL.
    #[serde(default, rename = "isolationId")]
    pub isolation_id: Option<i32>,
    /// Name of a specific virtual network adapter to target.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Apply the ACL to the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmNetworkAdapterExtendedAclInfo {
    #[serde(rename = "action")]
    pub action: String,
    #[serde(rename = "direction")]
    pub direction: String,
    #[serde(rename = "localIpAddress")]
    pub local_ip_address: String,
    #[serde(rename = "remoteIpAddress")]
    pub remote_ip_address: String,
    #[serde(rename = "localPort")]
    pub local_port: String,
    #[serde(rename = "remotePort")]
    pub remote_port: String,
    #[serde(rename = "protocol")]
    pub protocol: String,
    #[serde(rename = "weight")]
    pub weight: i32,
    #[serde(rename = "stateful")]
    pub stateful: bool,
    #[serde(rename = "idleSessionTimeout")]
    pub idle_session_timeout: i32,
    #[serde(rename = "isolationId")]
    pub isolation_id: i32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmNetworkAdapterExtendedAclOutput {
    pub acls: Vec<AddVmNetworkAdapterExtendedAclInfo>,
}

#[derive(Default)]
pub struct AddVmNetworkAdapterExtendedAclTool;

#[async_trait]
impl HyperVTool for AddVmNetworkAdapterExtendedAclTool {
    const NAME: &'static str = "hyperv_add_vm_network_adapter_extended_acl";
    const DESCRIPTION: &'static str = "Creates an extended ACL for a virtual network adapter.";
    type Input = AddVmNetworkAdapterExtendedAclInput;
    type Output = AddVmNetworkAdapterExtendedAclOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.action.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Action must not be empty".to_string(),
            ));
        }
        if input.direction.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Direction must not be empty".to_string(),
            ));
        }

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

        let mut args = vec!["Add-VMNetworkAdapterExtendedAcl".to_string()];

        if management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        args.push(format!("-Action '{}'", escape_ps_string(&input.action)));
        args.push(format!(
            "-Direction '{}'",
            escape_ps_string(&input.direction)
        ));
        args.push(format!("-Weight {}", input.weight));

        if let Some(local_ip) = &input.local_ip_address {
            if !local_ip.trim().is_empty() {
                args.push(format!("-LocalIPAddress '{}'", escape_ps_string(local_ip)));
            }
        }
        if let Some(remote_ip) = &input.remote_ip_address {
            if !remote_ip.trim().is_empty() {
                args.push(format!(
                    "-RemoteIPAddress '{}'",
                    escape_ps_string(remote_ip)
                ));
            }
        }
        if let Some(local_port) = &input.local_port {
            if !local_port.trim().is_empty() {
                args.push(format!("-LocalPort '{}'", escape_ps_string(local_port)));
            }
        }
        if let Some(remote_port) = &input.remote_port {
            if !remote_port.trim().is_empty() {
                args.push(format!("-RemotePort '{}'", escape_ps_string(remote_port)));
            }
        }
        if let Some(protocol) = &input.protocol {
            if !protocol.trim().is_empty() {
                args.push(format!("-Protocol '{}'", escape_ps_string(protocol)));
            }
        }
        if let Some(stateful) = input.stateful {
            args.push(format!("-Stateful ${}", stateful));
        }
        if let Some(timeout) = input.idle_session_timeout {
            args.push(format!("-IdleSessionTimeout {}", timeout));
        }
        if let Some(isolation_id) = input.isolation_id {
            args.push(format!("-IsolationID {}", isolation_id));
        }
        if let Some(adapter) = &input.vm_network_adapter_name {
            if !adapter.trim().is_empty() {
                args.push(format!(
                    "-VMNetworkAdapterName '{}'",
                    escape_ps_string(adapter)
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            if !computer.trim().is_empty() {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             @{{N='Action';E={{$_.Action.ToString()}}}}, \
             @{{N='Direction';E={{$_.Direction.ToString()}}}}, \
             LocalIPAddress, RemoteIPAddress, LocalPort, RemotePort, Protocol, \
             Weight, Stateful, IdleSessionTimeout, IsolationID | \
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
            output.push(AddVmNetworkAdapterExtendedAclInfo {
                action: acl["Action"].as_str().unwrap_or_default().to_string(),
                direction: acl["Direction"].as_str().unwrap_or_default().to_string(),
                local_ip_address: acl["LocalIPAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                remote_ip_address: acl["RemoteIPAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                local_port: acl["LocalPort"].as_str().unwrap_or_default().to_string(),
                remote_port: acl["RemotePort"].as_str().unwrap_or_default().to_string(),
                protocol: acl["Protocol"].as_str().unwrap_or_default().to_string(),
                weight: acl["Weight"].as_i64().unwrap_or_default() as i32,
                stateful: acl["Stateful"].as_bool().unwrap_or_default(),
                idle_session_timeout: acl["IdleSessionTimeout"].as_i64().unwrap_or_default() as i32,
                isolation_id: acl["IsolationID"].as_i64().unwrap_or_default() as i32,
            });
        }

        Ok(AddVmNetworkAdapterExtendedAclOutput { acls: output })
    }
}

register_tool!(AddVmNetworkAdapterExtendedAclTool);
