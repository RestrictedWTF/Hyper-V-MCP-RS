use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmReplicationInput {
    /// Name of the virtual machine whose replication relationship should be removed.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedReplicationInfo {
    pub name: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    pub state: String,
    pub health: String,
    pub mode: String,
    #[serde(rename = "primaryServer")]
    pub primary_server: String,
    #[serde(rename = "replicaServer")]
    pub replica_server: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmReplicationOutput {
    /// Replication relationships that were removed.
    pub removed: Vec<RemovedReplicationInfo>,
}

#[derive(Default)]
pub struct RemoveVmReplicationTool;

#[async_trait]
impl HyperVTool for RemoveVmReplicationTool {
    const NAME: &'static str = "hyperv_remove_vm_replication";
    const DESCRIPTION: &'static str = "Removes the replication relationship of a virtual machine.";
    type Input = RemoveVmReplicationInput;
    type Output = RemoveVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             Name, \
             VMName, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}}, \
             @{{N='Mode';E={{$_.Mode.ToString()}}}}, \
             PrimaryServerName, PrimaryServer, \
             ReplicaServerName, ReplicaServer | \
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

        let relationships = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut removed = Vec::with_capacity(relationships.len());
        for rel in relationships {
            let primary_server = rel["PrimaryServerName"]
                .as_str()
                .or_else(|| rel["PrimaryServer"].as_str())
                .unwrap_or_default()
                .to_string();
            let replica_server = rel["ReplicaServerName"]
                .as_str()
                .or_else(|| rel["ReplicaServer"].as_str())
                .unwrap_or_default()
                .to_string();

            removed.push(RemovedReplicationInfo {
                name: rel["Name"].as_str().unwrap_or_default().to_string(),
                vm_name: rel["VMName"].as_str().unwrap_or_default().to_string(),
                state: rel["State"].as_str().unwrap_or_default().to_string(),
                health: rel["Health"].as_str().unwrap_or_default().to_string(),
                mode: rel["Mode"].as_str().unwrap_or_default().to_string(),
                primary_server,
                replica_server,
            });
        }

        Ok(RemoveVmReplicationOutput { removed })
    }
}

register_tool!(RemoveVmReplicationTool);
