use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterTeamMappingInput {
    /// Name of the virtual machine whose network adapter team mapping should be removed.
    /// Not used when management_os is true.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual network adapter whose team mapping should be removed.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Operate on the management operating system instead of a VM.
    #[serde(default, rename = "managementOS")]
    pub management_os: Option<bool>,
    /// Name of the virtual switch. Only used with managementOS.
    #[serde(default, rename = "switchName")]
    pub switch_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmNetworkAdapterTeamMappingOutput {
    /// True if the cmdlet executed without a sidecar or JSON error.
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmNetworkAdapterTeamMappingTool;

#[async_trait]
impl HyperVTool for RemoveVmNetworkAdapterTeamMappingTool {
    const NAME: &'static str = "hyperv_remove_vm_network_adapter_team_mapping";
    const DESCRIPTION: &'static str =
        "Removes the team mapping settings from a virtual network adapter.";
    type Input = RemoveVmNetworkAdapterTeamMappingInput;
    type Output = RemoveVmNetworkAdapterTeamMappingOutput;

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

        let mut args = vec!["Remove-VMNetworkAdapterTeamMapping".to_string()];

        if management_os {
            args.push("-ManagementOS".to_string());
        } else if let Some(vm) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm)));
        }

        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(switch) = &input.switch_name {
            if switch.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Switch name must not be empty".to_string(),
                ));
            }
            args.push(format!("-SwitchName '{}'", escape_ps_string(switch)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = args.join(" ");

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let _items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        Ok(RemoveVmNetworkAdapterTeamMappingOutput { success: true })
    }
}

register_tool!(RemoveVmNetworkAdapterTeamMappingTool);
