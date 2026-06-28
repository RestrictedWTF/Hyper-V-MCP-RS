use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopVmReplicationInput {
    /// Name of the virtual machine whose ongoing resynchronization should be cancelled.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Replication relationship type, e.g. "Simple" or "Extended".
    #[serde(default, rename = "replicationRelationshipType")]
    pub replication_relationship_type: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StoppedVmReplicationInfo {
    pub vm_name: String,
    pub state: String,
    pub health: String,
    pub mode: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StopVmReplicationOutput {
    pub replication: Vec<StoppedVmReplicationInfo>,
}

#[derive(Default)]
pub struct StopVmReplicationTool;

#[async_trait]
impl HyperVTool for StopVmReplicationTool {
    const NAME: &'static str = "hyperv_stop_vm_replication";
    const DESCRIPTION: &'static str = "Cancels an ongoing virtual machine resynchronization.";
    type Input = StopVmReplicationInput;
    type Output = StopVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Stop-VMReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(relationship) = &input.replication_relationship_type {
            args.push(format!(
                "-ReplicationRelationshipType '{}'",
                escape_ps_string(relationship)
            ));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             @{{N='VMName';E={{$_.VMName}}}}, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}}, \
             @{{N='Mode';E={{$_.Mode.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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

        let mut replication = Vec::with_capacity(items.len());
        for item in items {
            replication.push(StoppedVmReplicationInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                state: item["State"].as_str().unwrap_or_default().to_string(),
                health: item["Health"].as_str().unwrap_or_default().to_string(),
                mode: item["Mode"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(StopVmReplicationOutput { replication })
    }
}

register_tool!(StopVmReplicationTool);
