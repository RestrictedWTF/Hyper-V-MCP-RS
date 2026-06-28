use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmResourceMeteringInput {
    /// Name of the virtual machine whose resource metering is to be disabled.
    #[serde(default)]
    pub name: Option<String>,
    /// Name of the resource pool whose resource metering is to be disabled.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisabledMeteringInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "resourcePoolName")]
    pub resource_pool_name: String,
    #[serde(rename = "meteringDuration")]
    pub metering_duration: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmResourceMeteringOutput {
    /// Resource metering entries that were disabled.
    pub disabled: Vec<DisabledMeteringInfo>,
}

#[derive(Default)]
pub struct DisableVmResourceMeteringTool;

#[async_trait]
impl HyperVTool for DisableVmResourceMeteringTool {
    const NAME: &'static str = "hyperv_disable_vm_resource_metering";
    const DESCRIPTION: &'static str =
        "Disables collection of resource utilization data for a virtual machine or resource pool.";
    type Input = DisableVmResourceMeteringInput;
    type Output = DisableVmResourceMeteringOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.as_ref().map(|s| s.trim()).unwrap_or("").is_empty()
            && input
                .resource_pool_name
                .as_ref()
                .map(|s| s.trim())
                .unwrap_or("")
                .is_empty()
        {
            return Err(ToolError::InvalidInput(
                "Either name or resourcePoolName must be provided".to_string(),
            ));
        }

        let mut args = vec!["Disable-VMResourceMetering".to_string()];

        if let Some(name) = &input.name {
            if !name.trim().is_empty() {
                args.push(format!("-VMName '{}'", escape_ps_string(name)));
            }
        }

        if let Some(resource_pool_name) = &input.resource_pool_name {
            if !resource_pool_name.trim().is_empty() {
                args.push(format!(
                    "-ResourcePoolName '{}'",
                    escape_ps_string(resource_pool_name)
                ));
            }
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object VMName, ResourcePoolName, \
             @{{N='MeteringDuration';E={{$_.MeteringDuration.ToString()}}}} | \
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

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut disabled = Vec::with_capacity(items.len());
        for item in items {
            disabled.push(DisabledMeteringInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                resource_pool_name: item["ResourcePoolName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                metering_duration: item["MeteringDuration"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(DisableVmResourceMeteringOutput { disabled })
    }
}

register_tool!(DisableVmResourceMeteringTool);
