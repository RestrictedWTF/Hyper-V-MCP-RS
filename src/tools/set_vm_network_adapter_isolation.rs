use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmNetworkAdapterIsolationInput {
    /// Name of the virtual machine whose network adapter isolation is to be configured.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to configure.
    #[serde(default, rename = "vmNetworkAdapterName")]
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Configure the adapter in the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Isolation mode for the network adapter. Valid values: None, NativeVirtualSubnet, ExternalVirtualSubnet, Vlan.
    #[serde(default, rename = "isolationMode")]
    pub isolation_mode: Option<String>,
    /// Allow untagged traffic on the adapter.
    #[serde(default, rename = "allowUntaggedTraffic")]
    pub allow_untagged_traffic: Option<bool>,
    /// Default isolation ID for the adapter.
    #[serde(default, rename = "defaultIsolationID")]
    pub default_isolation_id: Option<i32>,
    /// Enable or disable the multi-tenant stack. Valid values: On, Off.
    #[serde(default, rename = "multiTenantStack")]
    pub multi_tenant_stack: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterIsolationInfo {
    #[serde(rename = "isolationMode")]
    pub isolation_mode: String,
    #[serde(rename = "allowUntaggedTraffic")]
    pub allow_untagged_traffic: bool,
    #[serde(rename = "defaultIsolationID")]
    pub default_isolation_id: i32,
    #[serde(rename = "multiTenantStack")]
    pub multi_tenant_stack: String,
    #[serde(rename = "adapterName")]
    pub adapter_name: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmNetworkAdapterIsolationOutput {
    pub settings: Vec<VmNetworkAdapterIsolationInfo>,
}

#[derive(Default)]
pub struct SetVmNetworkAdapterIsolationTool;

#[async_trait]
impl HyperVTool for SetVmNetworkAdapterIsolationTool {
    const NAME: &'static str = "hyperv_set_vm_network_adapter_isolation";
    const DESCRIPTION: &'static str = "Modifies isolation settings for a virtual network adapter.";
    type Input = SetVmNetworkAdapterIsolationInput;
    type Output = SetVmNetworkAdapterIsolationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Network adapter name must not be empty".to_string(),
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

        if input.isolation_mode.is_none()
            && input.allow_untagged_traffic.is_none()
            && input.default_isolation_id.is_none()
            && input.multi_tenant_stack.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one isolation setting must be provided".to_string(),
            ));
        }

        let mut args = vec!["Set-VMNetworkAdapterIsolation".to_string()];

        if management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        args.push(format!(
            "-VMNetworkAdapterName '{}'",
            escape_ps_string(&input.name)
        ));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(mode) = &input.isolation_mode {
            args.push(format!("-IsolationMode '{}'", escape_ps_string(mode)));
        }
        if let Some(allowed) = input.allow_untagged_traffic {
            args.push(format!("-AllowUntaggedTraffic ${}", allowed));
        }
        if let Some(id) = input.default_isolation_id {
            args.push(format!("-DefaultIsolationID {}", id));
        }
        if let Some(stack) = &input.multi_tenant_stack {
            args.push(format!("-MultiTenantStack '{}'", escape_ps_string(stack)));
        }


        let ps = format!(
            "{} | Select-Object \
             @{{N='IsolationMode';E={{$_.IsolationMode.ToString()}}}}, \
             AllowUntaggedTraffic, DefaultIsolationID, \
             @{{N='MultiTenantStack';E={{$_.MultiTenantStack.ToString()}}}}, \
             @{{N='AdapterName';E={{$_.ParentAdapter.Name}}}}, \
             @{{N='VMName';E={{$_.ParentAdapter.VMName}}}}, \
             ComputerName | ConvertTo-Json -Compress -Depth 3",
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
                adapter_name: setting["AdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_name: setting["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: setting["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmNetworkAdapterIsolationOutput { settings: output })
    }
}

register_tool!(SetVmNetworkAdapterIsolationTool);
