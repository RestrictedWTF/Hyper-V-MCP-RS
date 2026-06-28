use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SuspendVmReplicationInput {
    /// Name of the virtual machine whose replication is to be suspended.
pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Replication relationship type: Simple or Extended.
    #[serde(default, rename = "replicationRelationshipType")]
    pub replication_relationship_type: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SuspendedVmReplicationInfo {
    pub vm_name: String,
    pub replication_state: String,
    pub replication_health: String,
    pub replication_mode: String,
    pub replication_relationship_type: String,
    pub primary_server_name: String,
    pub replica_server_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SuspendVmReplicationOutput {
    pub replications: Vec<SuspendedVmReplicationInfo>,
}

#[derive(Default)]
pub struct SuspendVmReplicationTool;

#[async_trait]
impl HyperVTool for SuspendVmReplicationTool {
    const NAME: &'static str = "hyperv_suspend_vm_replication";
    const DESCRIPTION: &'static str = "Suspends replication of a virtual machine.";
    type Input = SuspendVmReplicationInput;
    type Output = SuspendVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Suspend-VMReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(relationship) = &input.replication_relationship_type {
            let relationship = relationship.trim();
            if relationship.is_empty() {
                return Err(ToolError::InvalidInput(
                    "replication_relationship_type must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ReplicationRelationshipType '{}'",
                escape_ps_string(relationship)
            ));
        }


        let ps = format!(
            "{} | Select-Object VMName, \
             @{{N='ReplicationState';E={{$_.ReplicationState.ToString()}}}}, \
             @{{N='ReplicationHealth';E={{$_.ReplicationHealth.ToString()}}}}, \
             @{{N='ReplicationMode';E={{$_.ReplicationMode.ToString()}}}}, \
             @{{N='ReplicationRelationshipType';E={{$_.ReplicationRelationshipType.ToString()}}}}, \
             PrimaryServerName, ReplicaServerName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let replications = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(replications.len());
        for replication in replications {
            output.push(SuspendedVmReplicationInfo {
                vm_name: replication["VMName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_state: replication["ReplicationState"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_health: replication["ReplicationHealth"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_mode: replication["ReplicationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_relationship_type: replication["ReplicationRelationshipType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                primary_server_name: replication["PrimaryServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_name: replication["ReplicaServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SuspendVmReplicationOutput { replications: output })
    }
}

register_tool!(SuspendVmReplicationTool);
