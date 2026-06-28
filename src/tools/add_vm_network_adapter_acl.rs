use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmNetworkAdapterAclInput {
    /// Name of the virtual machine on which the ACL is to be created.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Action for the ACL. Valid values: Allow, Deny, Meter.
    pub action: String,
    /// Direction of network traffic. Valid values: Inbound, Outbound, Both.
    pub direction: String,
    /// Local IP address to which the ACL applies (host address, subnet, or ANY).
    #[serde(default, rename = "localIPAddress")]
    pub local_ip_address: Option<String>,
    /// Local MAC address to which the ACL applies (host MAC or ANY).
    #[serde(default, rename = "localMACAddress")]
    pub local_mac_address: Option<String>,
    /// Remote IP address to which the ACL applies (host address, subnet, or ANY).
    #[serde(default, rename = "remoteIPAddress")]
    pub remote_ip_address: Option<String>,
    /// Remote MAC address to which the ACL applies (host MAC or ANY).
    #[serde(default, rename = "remoteMACAddress")]
    pub remote_mac_address: Option<String>,
    /// Name of the virtual network adapter to which the ACL applies.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Apply the ACL in the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterAclInfo {
    #[serde(rename = "localAddress")]
    pub local_address: String,
    #[serde(rename = "localAddressType")]
    pub local_address_type: String,
    #[serde(rename = "remoteAddress")]
    pub remote_address: String,
    #[serde(rename = "remoteAddressType")]
    pub remote_address_type: String,
    pub action: String,
    pub direction: String,
    #[serde(rename = "meteredMegabytes")]
    pub metered_megabytes: String,
    #[serde(rename = "isTemplate")]
    pub is_template: bool,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmNetworkAdapterAclOutput {
    pub acls: Vec<VmNetworkAdapterAclInfo>,
}

#[derive(Default)]
pub struct AddVmNetworkAdapterAclTool;

#[async_trait]
impl HyperVTool for AddVmNetworkAdapterAclTool {
    const NAME: &'static str = "hyperv_add_vm_network_adapter_acl";
    const DESCRIPTION: &'static str =
        "Creates an ACL to apply to the traffic through a virtual machine network adapter.";
    type Input = AddVmNetworkAdapterAclInput;
    type Output = AddVmNetworkAdapterAclOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.action.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Action must not be empty".to_string(),
            ));
        }
        let action_lower = input.action.trim().to_lowercase();
        if action_lower != "allow" && action_lower != "deny" && action_lower != "meter" {
            return Err(ToolError::InvalidInput(
                "Action must be one of: Allow, Deny, Meter".to_string(),
            ));
        }

        if input.direction.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Direction must not be empty".to_string(),
            ));
        }
        let direction_lower = input.direction.trim().to_lowercase();
        if direction_lower != "inbound"
            && direction_lower != "outbound"
            && direction_lower != "both"
        {
            return Err(ToolError::InvalidInput(
                "Direction must be one of: Inbound, Outbound, Both".to_string(),
            ));
        }

        if action_lower == "meter"
            && input.local_ip_address.is_none()
            && input.remote_ip_address.is_none()
        {
            return Err(ToolError::InvalidInput(
                "A metering ACL must specify -LocalIPAddress or -RemoteIPAddress".to_string(),
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

        if input.local_ip_address.is_none()
            && input.local_mac_address.is_none()
            && input.remote_ip_address.is_none()
            && input.remote_mac_address.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one address filter must be provided (local_ip_address, local_mac_address, remote_ip_address, or remote_mac_address)".to_string(),
            ));
        }

        let mut args = vec!["Add-VMNetworkAdapterAcl".to_string()];

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

        if let Some(addr) = &input.local_ip_address {
            if addr.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Local IP address must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-LocalIPAddress '{}'", escape_ps_string(addr)));
        }
        if let Some(addr) = &input.local_mac_address {
            if addr.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Local MAC address must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-LocalMacAddress '{}'", escape_ps_string(addr)));
        }
        if let Some(addr) = &input.remote_ip_address {
            if addr.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Remote IP address must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-RemoteIPAddress '{}'", escape_ps_string(addr)));
        }
        if let Some(addr) = &input.remote_mac_address {
            if addr.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Remote MAC address must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-RemoteMacAddress '{}'", escape_ps_string(addr)));
        }
        if let Some(adapter) = &input.vm_network_adapter_name {
            if adapter.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM network adapter name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(adapter)
            ));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             LocalAddress, \
             @{{N='LocalAddressType';E={{$_.LocalAddressType.ToString()}}}}, \
             RemoteAddress, \
             @{{N='RemoteAddressType';E={{$_.RemoteAddressType.ToString()}}}}, \
             @{{N='Action';E={{$_.Action.ToString()}}}}, \
             @{{N='Direction';E={{$_.Direction.ToString()}}}}, \
             MeteredMegabytes, IsTemplate, IsDeleted, ComputerName | \
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
            output.push(VmNetworkAdapterAclInfo {
                local_address: acl["LocalAddress"].as_str().unwrap_or_default().to_string(),
                local_address_type: acl["LocalAddressType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                remote_address: acl["RemoteAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                remote_address_type: acl["RemoteAddressType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                action: acl["Action"].as_str().unwrap_or_default().to_string(),
                direction: acl["Direction"].as_str().unwrap_or_default().to_string(),
                metered_megabytes: acl["MeteredMegabytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_template: acl["IsTemplate"].as_bool().unwrap_or_default(),
                is_deleted: acl["IsDeleted"].as_bool().unwrap_or_default(),
                computer_name: acl["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(AddVmNetworkAdapterAclOutput { acls: output })
    }
}

register_tool!(AddVmNetworkAdapterAclTool);
