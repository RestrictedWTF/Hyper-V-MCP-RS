use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmIntegrationServiceInfo {
    pub name: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "primaryStatus")]
    pub primary_status: String,
    #[serde(rename = "primaryStatusDescription")]
    pub primary_status_description: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmIntegrationServiceInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmIntegrationServiceOutput {
    pub services: Vec<VmIntegrationServiceInfo>,
}

#[derive(Default)]
pub struct GetVmIntegrationServiceTool;

#[async_trait]
impl HyperVTool for GetVmIntegrationServiceTool {
    const NAME: &'static str = "hyperv_get_vm_integration_service";
    const DESCRIPTION: &'static str =
        "Gets the integration services of a virtual machine or snapshot.";
    type Input = GetVmIntegrationServiceInput;
    type Output = GetVmIntegrationServiceOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMIntegrationService".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = format!("{} | Select-Object Name, VMName, VMId, Enabled, @{{N='PrimaryStatus';E={{$_.PrimaryStatus.ToString()}}}}, @{{N='PrimaryStatusDescription';E={{$_.PrimaryStatusDescription.ToString()}}}} | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            output.push(VmIntegrationServiceInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                enabled: item["Enabled"].as_bool().unwrap_or_default(),
                primary_status: item["PrimaryStatus"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                primary_status_description: item["PrimaryStatusDescription"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmIntegrationServiceOutput { services: output })
    }
}

register_tool!(GetVmIntegrationServiceTool);
