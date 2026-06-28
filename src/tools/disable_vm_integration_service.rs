use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmIntegrationServiceInput {
    /// Name of the virtual machine on which to disable the integration service.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the integration service to disable (e.g. "VSS", "Shutdown",
    /// "Time Synchronization", "Heartbeat", "Key-Value Pair Exchange",
    /// "Guest Service Interface").
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmIntegrationServiceInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(rename = "primaryStatusDescription")]
    pub primary_status_description: String,
    #[serde(rename = "secondaryStatusDescription")]
    pub secondary_status_description: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmIntegrationServiceOutput {
    /// Integration services that were disabled.
    pub services: Vec<VmIntegrationServiceInfo>,
}

#[derive(Default)]
pub struct DisableVmIntegrationServiceTool;

#[async_trait]
impl HyperVTool for DisableVmIntegrationServiceTool {
    const NAME: &'static str = "hyperv_disable_vm_integration_service";
    const DESCRIPTION: &'static str = "Disables an integration service on a virtual machine.";
    type Input = DisableVmIntegrationServiceInput;
    type Output = DisableVmIntegrationServiceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Disable-VMIntegrationService".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }


        let ps = format!(
            "{} | Select-Object VMName, VMId, Name, Enabled, \
             PrimaryStatusDescription, SecondaryStatusDescription | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let services = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(services.len());
        for service in services {
            output.push(VmIntegrationServiceInfo {
                vm_name: service["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: service["VMId"].as_str().unwrap_or_default().to_string(),
                name: service["Name"].as_str().unwrap_or_default().to_string(),
                enabled: service["Enabled"].as_bool().unwrap_or_default(),
                primary_status_description: service["PrimaryStatusDescription"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                secondary_status_description: service["SecondaryStatusDescription"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(DisableVmIntegrationServiceOutput { services: output })
    }
}

register_tool!(DisableVmIntegrationServiceTool);
