use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmNetworkAdapterTeamMappingInput {
    /// Name of the virtual machine whose virtual network adapter is to be mapped.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter to map to a physical network adapter.
    #[serde(rename = "vmNetworkAdapterName")]
    pub name: String,
    /// Name of the physical network adapter to map the virtual network adapter to.
    #[serde(rename = "physicalNetAdapterName")]
    pub physical_net_adapter_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Configure team mapping for the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Name of the virtual switch. Only used with management_os.
    #[serde(default, rename = "switchName")]
    pub switch_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmNetworkAdapterTeamMappingInfo {
    #[serde(rename = "vmNetworkAdapterName")]
    pub name: String,
    #[serde(rename = "switchName")]
    pub switch_name: String,
    #[serde(rename = "netAdapterName")]
    pub net_adapter_name: String,
    #[serde(rename = "netAdapterInterfaceDescription")]
    pub net_adapter_interface_description: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmNetworkAdapterTeamMappingOutput {
    pub mappings: Vec<VmNetworkAdapterTeamMappingInfo>,
}

#[derive(Default)]
pub struct SetVmNetworkAdapterTeamMappingTool;

#[async_trait]
impl HyperVTool for SetVmNetworkAdapterTeamMappingTool {
    const NAME: &'static str = "hyperv_set_vm_network_adapter_team_mapping";
    const DESCRIPTION: &'static str =
        "Configures team mapping settings for a virtual network adapter.";
    type Input = SetVmNetworkAdapterTeamMappingInput;
    type Output = SetVmNetworkAdapterTeamMappingOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Virtual network adapter name must not be empty".to_string(),
            ));
        }
        if input.physical_net_adapter_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Physical network adapter name must not be empty".to_string(),
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

        let mut args = vec!["Set-VMNetworkAdapterTeamMapping".to_string()];

        if management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        args.push(format!(
            "-VMNetworkAdapterName '{}'",
            escape_ps_string(&input.name)
        ));
        args.push(format!(
            "-PhysicalNetAdapterName '{}'",
            escape_ps_string(&input.physical_net_adapter_name)
        ));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(switch) = &input.switch_name {
            args.push(format!("-SwitchName '{}'", escape_ps_string(switch)));
        }

        let ps = format!(
            "{} | Select-Object \
             @{{N='Name';E={{$_.Name}}}}, \
             SwitchName, \
             NetAdapterName, \
             NetAdapterInterfaceDescription, \
             VMName, \
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
            output.push(VmNetworkAdapterTeamMappingInfo {
                name: mapping["Name"].as_str().unwrap_or_default().to_string(),
                switch_name: mapping["SwitchName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                net_adapter_name: mapping["NetAdapterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                net_adapter_interface_description: mapping["NetAdapterInterfaceDescription"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_name: mapping["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: mapping["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmNetworkAdapterTeamMappingOutput { mappings: output })
    }
}

register_tool!(SetVmNetworkAdapterTeamMappingTool);
