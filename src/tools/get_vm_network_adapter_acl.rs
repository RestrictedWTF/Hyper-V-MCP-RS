use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterAclInput {
    /// Name of the virtual machine whose network adapter ACLs are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter whose ACLs are to be retrieved.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve ACLs configured in the management operating system.
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
pub struct GetVmNetworkAdapterAclOutput {
    pub acls: Vec<VmNetworkAdapterAclInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterAclTool;

#[async_trait]
impl HyperVTool for GetVmNetworkAdapterAclTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter_acl";
    const DESCRIPTION: &'static str =
        "Gets the ACLs configured for a virtual machine network adapter.";
    type Input = GetVmNetworkAdapterAclInput;
    type Output = GetVmNetworkAdapterAclOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.is_some() && input.management_os {
            return Err(ToolError::InvalidInput(
                "vm_name cannot be combined with management_os".to_string(),
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

        let mut args = vec!["Get-VMNetworkAdapterAcl".to_string()];

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

        let mut output = Vec::with_capacity(acls.len());
        for acl in acls {
            output.push(VmNetworkAdapterAclInfo {
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

        Ok(GetVmNetworkAdapterAclOutput { acls: output })
    }
}

register_tool!(GetVmNetworkAdapterAclTool);
