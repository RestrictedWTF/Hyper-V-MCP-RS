use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterAclInput {
    /// Name of the virtual machine whose network adapter ACL should be removed.
    /// Required unless management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter whose ACL should be removed.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Action of the ACL to remove. Valid values: Allow, Deny, Meter.
    pub action: String,
    /// Direction of traffic for the ACL to remove. Valid values: Inbound, Outbound, Both.
    pub direction: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Local IP address(es) of the ACL to remove.
    #[serde(default, rename = "localIpAddress")]
    pub local_ip_address: Option<Vec<String>>,
    /// Local MAC address(es) of the ACL to remove.
    #[serde(default, rename = "localMacAddress")]
    pub local_mac_address: Option<Vec<String>>,
    /// Remote IP address(es) of the ACL to remove.
    #[serde(default, rename = "remoteIpAddress")]
    pub remote_ip_address: Option<Vec<String>>,
    /// Remote MAC address(es) of the ACL to remove.
    #[serde(default, rename = "remoteMacAddress")]
    pub remote_mac_address: Option<Vec<String>>,
    /// Remove the ACL from the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterAclInfo {
    pub action: String,
    pub direction: String,
    #[serde(rename = "localAddress")]
    pub local_address: String,
    #[serde(rename = "remoteAddress")]
    pub remote_address: String,
    #[serde(rename = "localAddressType")]
    pub local_address_type: String,
    #[serde(rename = "remoteAddressType")]
    pub remote_address_type: String,
    #[serde(rename = "meteredMegabytes")]
    pub metered_megabytes: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
    #[serde(rename = "isTemplate")]
    pub is_template: bool,
    #[serde(rename = "adapterName")]
    pub adapter_name: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "adapterId")]
    pub adapter_id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterAclOutput {
    /// ACL entries that were removed.
    pub removed: Vec<VmNetworkAdapterAclInfo>,
}

#[derive(Default)]
pub struct RemoveVmNetworkAdapterAclTool;

fn push_string_array_arg(args: &mut Vec<String>, flag: &str, values: &[String]) {
    let escaped: Vec<String> = values
        .iter()
        .map(|v| format!("'{}'", escape_ps_string(v)))
        .collect();
    args.push(format!("-{} {}", flag, escaped.join(",")));
}

#[async_trait]
impl HyperVTool for RemoveVmNetworkAdapterAclTool {
    const NAME: &'static str = "hyperv_remove_vm_network_adapter_acl";
    const DESCRIPTION: &'static str =
        "Removes an ACL applied to the traffic through a virtual network adapter.";
    type Input = RemoveVmNetworkAdapterAclInput;
    type Output = RemoveVmNetworkAdapterAclOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.is_some() && input.management_os {
            return Err(ToolError::InvalidInput(
                "vm_name cannot be combined with management_os".to_string(),
            ));
        }

        if !input.management_os && input.vm_name.is_none() {
            return Err(ToolError::InvalidInput(
                "vm_name must be provided when management_os is not enabled".to_string(),
            ));
        }

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
        }
        if let Some(adapter_name) = &input.vm_network_adapter_name {
            if adapter_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty".to_string(),
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
        }

        let action = input.action.trim();
        if action.is_empty() {
            return Err(ToolError::InvalidInput(
                "action must not be empty".to_string(),
            ));
        }
        if !matches!(
            action.to_ascii_lowercase().as_str(),
            "allow" | "deny" | "meter"
        ) {
            return Err(ToolError::InvalidInput(
                "action must be one of: Allow, Deny, Meter".to_string(),
            ));
        }

        let direction = input.direction.trim();
        if direction.is_empty() {
            return Err(ToolError::InvalidInput(
                "direction must not be empty".to_string(),
            ));
        }
        if !matches!(
            direction.to_ascii_lowercase().as_str(),
            "inbound" | "outbound" | "both"
        ) {
            return Err(ToolError::InvalidInput(
                "direction must be one of: Inbound, Outbound, Both".to_string(),
            ));
        }

        if let Some(addresses) = &input.local_ip_address {
            if addresses.is_empty() {
                return Err(ToolError::InvalidInput(
                    "localIpAddress must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(addresses) = &input.local_mac_address {
            if addresses.is_empty() {
                return Err(ToolError::InvalidInput(
                    "localMacAddress must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(addresses) = &input.remote_ip_address {
            if addresses.is_empty() {
                return Err(ToolError::InvalidInput(
                    "remoteIpAddress must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(addresses) = &input.remote_mac_address {
            if addresses.is_empty() {
                return Err(ToolError::InvalidInput(
                    "remoteMacAddress must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec!["Remove-VMNetworkAdapterAcl".to_string()];

        if input.management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm_name) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }

        if let Some(adapter_name) = &input.vm_network_adapter_name {
            args.push(format!(
                "-VMNetworkAdapterName '{}'",
                escape_ps_string(adapter_name)
            ));
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push(format!("-Action '{}'", escape_ps_string(action)));
        args.push(format!("-Direction '{}'", escape_ps_string(direction)));

        if let Some(addresses) = &input.local_ip_address {
            push_string_array_arg(&mut args, "LocalIPAddress", addresses);
        }
        if let Some(addresses) = &input.local_mac_address {
            push_string_array_arg(&mut args, "LocalMacAddress", addresses);
        }
        if let Some(addresses) = &input.remote_ip_address {
            push_string_array_arg(&mut args, "RemoteIPAddress", addresses);
        }
        if let Some(addresses) = &input.remote_mac_address {
            push_string_array_arg(&mut args, "RemoteMacAddress", addresses);
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             @{{N='Action';E={{$_.Action.ToString()}}}}, \
             @{{N='Direction';E={{$_.Direction.ToString()}}}}, \
             LocalAddress, RemoteAddress, \
             @{{N='LocalAddressType';E={{$_.LocalAddressType.ToString()}}}}, \
             @{{N='RemoteAddressType';E={{$_.RemoteAddressType.ToString()}}}}, \
             MeteredMegabytes, ComputerName, IsDeleted, IsTemplate, \
             @{{N='AdapterName';E={{$_.ParentAdapter.Name}}}}, \
             @{{N='VMName';E={{$_.ParentAdapter.VMName}}}}, \
             @{{N='VMId';E={{$_.ParentAdapter.VMId.ToString()}}}}, \
             @{{N='AdapterId';E={{$_.ParentAdapter.AdapterId}}}} | \
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
            removed.push(VmNetworkAdapterAclInfo {
                action: acl["Action"].as_str().unwrap_or_default().to_string(),
                direction: acl["Direction"].as_str().unwrap_or_default().to_string(),
                local_address: acl["LocalAddress"].as_str().unwrap_or_default().to_string(),
                remote_address: acl["RemoteAddress"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                local_address_type: acl["LocalAddressType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                remote_address_type: acl["RemoteAddressType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                metered_megabytes: acl["MeteredMegabytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: acl["ComputerName"].as_str().unwrap_or_default().to_string(),
                is_deleted: acl["IsDeleted"].as_bool().unwrap_or_default(),
                is_template: acl["IsTemplate"].as_bool().unwrap_or_default(),
                adapter_name: acl["AdapterName"].as_str().unwrap_or_default().to_string(),
                vm_name: acl["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: acl["VMId"].as_str().unwrap_or_default().to_string(),
                adapter_id: acl["AdapterId"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(RemoveVmNetworkAdapterAclOutput { removed })
    }
}

register_tool!(RemoveVmNetworkAdapterAclTool);
