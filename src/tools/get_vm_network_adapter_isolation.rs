use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmNetworkAdapterIsolationInput {
    /// Name of the virtual machine whose adapter isolation settings are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter whose isolation settings are to be retrieved.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Retrieve isolation settings for adapters of the management operating system.
    #[serde(default, rename = "managementOS")]
    pub management_os: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterIsolationInfo {
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    #[serde(rename = "vmNetworkAdapterName")]
    pub vm_network_adapter_name: String,
    #[serde(rename = "isolationMode")]
    pub isolation_mode: String,
    pub allow_untagged_traffic: bool,
    #[serde(rename = "defaultIsolationId")]
    pub default_isolation_id: i32,
    #[serde(rename = "multiTenantStack")]
    pub multi_tenant_stack: String,
    #[serde(rename = "parentAdapter")]
    pub parent_adapter: String,
    pub is_template: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmNetworkAdapterIsolationOutput {
    pub isolation_settings: Vec<VmNetworkAdapterIsolationInfo>,
}

#[derive(Default)]
pub struct GetVmNetworkAdapterIsolationTool;

#[async_trait]
impl HyperVTool for GetVmNetworkAdapterIsolationTool {
    const NAME: &'static str = "hyperv_get_vm_network_adapter_isolation";
    const DESCRIPTION: &'static str = "Gets isolation settings for a virtual network adapter.";
    type Input = GetVmNetworkAdapterIsolationInput;
    type Output = GetVmNetworkAdapterIsolationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.management_os && input.vm_name.is_some() {
            return Err(ToolError::InvalidInput(
                "management_os cannot be combined with vm_name".to_string(),
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

        let mut args = vec!["Get-VMNetworkAdapterIsolation".to_string()];

        if input.management_os {
            args.push("-ManagementOS".to_string());
        }
        if let Some(vm_name) = &input.vm_name {
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
             @{{N='VMName';E={{$_.ParentAdapter.VMName}}}}, \
             @{{N='VMNetworkAdapterName';E={{$_.ParentAdapter.Name}}}}, \
             @{{N='IsolationMode';E={{$_.IsolationMode.ToString()}}}}, \
             AllowUntaggedTraffic, DefaultIsolationID, \
             @{{N='MultiTenantStack';E={{$_.MultiTenantStack.ToString()}}}}, \
             @{{N='ParentAdapter';E={{$_.ParentAdapter.ToString()}}}}, \
             IsTemplate, ComputerName, IsDeleted | \
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

        let settings = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(settings.len());
        for setting in settings {
            output.push(VmNetworkAdapterIsolationInfo {
                vm_name: setting["VMName"].as_str().map(String::from),
                vm_network_adapter_name: setting["VMNetworkAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                isolation_mode: setting["IsolationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                allow_untagged_traffic: setting["AllowUntaggedTraffic"]
                    .as_bool()
                    .unwrap_or_default(),
                default_isolation_id: setting["DefaultIsolationID"].as_i64().unwrap_or_default()
                    as i32,
                multi_tenant_stack: setting["MultiTenantStack"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_adapter: setting["ParentAdapter"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_template: setting["IsTemplate"].as_bool().unwrap_or_default(),
                computer_name: setting["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: setting["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmNetworkAdapterIsolationOutput {
            isolation_settings: output,
        })
    }
}

register_tool!(GetVmNetworkAdapterIsolationTool);
