use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmReplicationInput {
    /// Name of the virtual machine whose replication is to be disabled.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisabledReplicationInfo {
    pub vm_name: String,
    pub vm_id: String,
    pub state: String,
    pub health: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmReplicationOutput {
    /// Replication entries that were disabled.
    pub disabled: Vec<DisabledReplicationInfo>,
}

#[derive(Default)]
pub struct DisableVmReplicationTool;

#[async_trait]
impl HyperVTool for DisableVmReplicationTool {
    const NAME: &'static str = "hyperv_disable_vm_replication";
    const DESCRIPTION: &'static str = "Disables replication of a virtual machine.";
    type Input = DisableVmReplicationInput;
    type Output = DisableVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Disable-VMReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object VMName, VMId, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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
            disabled.push(DisabledReplicationInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                state: item["State"].as_str().unwrap_or_default().to_string(),
                health: item["Health"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(DisableVmReplicationOutput { disabled })
    }
}

register_tool!(DisableVmReplicationTool);
