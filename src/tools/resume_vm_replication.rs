use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResumeVmReplicationInput {
    /// Name of the virtual machine whose replication to resume.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Replication relationship type: Simple or Extended.
    #[serde(default, rename = "replicationRelationshipType")]
    pub replication_relationship_type: Option<String>,
    /// Resynchronizes the replica virtual machine.
    #[serde(default)]
    pub resynchronize: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationInfo {
    #[serde(rename = "VMName")]
    pub vm_name: String,
    #[serde(rename = "ComputerName")]
    pub computer_name: String,
    #[serde(rename = "PrimaryServerName")]
    pub primary_server_name: String,
    #[serde(rename = "ReplicaServerName")]
    pub replica_server_name: String,
    #[serde(rename = "ReplicationState")]
    pub replication_state: String,
    #[serde(rename = "ReplicationHealth")]
    pub replication_health: String,
    #[serde(rename = "ReplicationMode")]
    pub replication_mode: String,
    #[serde(rename = "ReplicationRelationshipType")]
    pub replication_relationship_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ResumeVmReplicationOutput {
    pub replications: Vec<VmReplicationInfo>,
}

#[derive(Default)]
pub struct ResumeVmReplicationTool;

#[async_trait]
impl HyperVTool for ResumeVmReplicationTool {
    const NAME: &'static str = "hyperv_resume_vm_replication";
    const DESCRIPTION: &'static str =
        "Resumes a virtual machine replication that is in a state of Paused, Error, Resynchronization Required, or Suspended.";
    type Input = ResumeVmReplicationInput;
    type Output = ResumeVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Resume-VMReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(relationship) = &input.replication_relationship_type {
            args.push(format!(
                "-ReplicationRelationshipType '{}'",
                escape_ps_string(relationship)
            ));
        }

        if input.resynchronize {
            args.push("-Resynchronize".to_string());
        }


        let ps = format!(
            "{} | Select-Object VMName, ComputerName, PrimaryServerName, ReplicaServerName, \
             @{{N='ReplicationState';E={{$_.ReplicationState.ToString()}}}}, \
             @{{N='ReplicationHealth';E={{$_.ReplicationHealth.ToString()}}}}, \
             @{{N='ReplicationMode';E={{$_.ReplicationMode.ToString()}}}}, \
             @{{N='ReplicationRelationshipType';E={{$_.ReplicationRelationshipType.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let reps = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(reps.len());
        for rep in reps {
            output.push(VmReplicationInfo {
                vm_name: rep["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: rep["ComputerName"].as_str().unwrap_or_default().to_string(),
                primary_server_name: rep["PrimaryServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_name: rep["ReplicaServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_state: rep["ReplicationState"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_health: rep["ReplicationHealth"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_mode: rep["ReplicationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_relationship_type: rep["ReplicationRelationshipType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(ResumeVmReplicationOutput { replications: output })
    }
}

register_tool!(ResumeVmReplicationTool);
