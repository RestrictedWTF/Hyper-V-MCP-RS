use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MeasureVmReplicationInput {
    /// Name of the virtual machine whose replication statistics are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the virtual machine. Alias for VMName.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Replication mode to filter on (e.g. Primary, Replica, TestReplica, ExtendedReplica).
    #[serde(default, rename = "replicationMode")]
    pub replication_mode: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationHealthInfo {
    pub vm_name: String,
    pub name: String,
    pub vm_id: String,
    pub replication_health: String,
    pub replication_mode: String,
    pub replication_state: String,
    pub last_replication_time: String,
    pub last_application_consistent_replication_time: String,
    pub last_normal_replication_time: String,
    pub average_replication_latency: u64,
    pub average_replication_size: u64,
    pub last_replication_size: u64,
    pub maximum_replication_size: u64,
    pub pending_replication_size: u64,
    pub successful_replication_count: u64,
    pub failed_replication_count: u64,
    pub missed_replication_count: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MeasureVmReplicationOutput {
    pub replication_health: Vec<VmReplicationHealthInfo>,
}

#[derive(Default)]
pub struct MeasureVmReplicationTool;

#[async_trait]
impl HyperVTool for MeasureVmReplicationTool {
    const NAME: &'static str = "hyperv_measure_vm_replication";
    const DESCRIPTION: &'static str =
        "Gets replication statistics and information associated with a virtual machine.";
    type Input = MeasureVmReplicationInput;
    type Output = MeasureVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Measure-VMReplication".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        } else if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(mode) = &input.replication_mode {
            if mode.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Replication mode must not be empty".to_string(),
                ));
            }
            args.push(format!("-ReplicationMode '{}'", escape_ps_string(mode)));
        }

        let ps = format!(
            "{} | Select-Object VMName, Name, \
             @{{N='VMId';E={{$_.VMId.ToString()}}}}, \
             @{{N='ReplicationHealth';E={{$_.ReplicationHealth.ToString()}}}}, \
             @{{N='ReplicationMode';E={{$_.ReplicationMode.ToString()}}}}, \
             @{{N='ReplicationState';E={{$_.ReplicationState.ToString()}}}}, \
             @{{N='LastReplicationTime';E={{$_.LastReplicationTime.ToString()}}}}, \
             @{{N='LastApplicationConsistentReplicationTime';E={{$_.LastApplicationConsistentReplicationTime.ToString()}}}}, \
             @{{N='LastNormalReplicationTime';E={{$_.LastNormalReplicationTime.ToString()}}}}, \
             AverageReplicationLatency, AverageReplicationSize, LastReplicationSize, \
             MaximumReplicationSize, PendingReplicationSize, SuccessfulReplicationCount, \
             FailedReplicationCount, MissedReplicationCount | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let health_entries = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(health_entries.len());
        for entry in health_entries {
            output.push(VmReplicationHealthInfo {
                vm_name: entry["VMName"].as_str().unwrap_or_default().to_string(),
                name: entry["Name"].as_str().unwrap_or_default().to_string(),
                vm_id: entry["VMId"].as_str().unwrap_or_default().to_string(),
                replication_health: entry["ReplicationHealth"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_mode: entry["ReplicationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_state: entry["ReplicationState"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                last_replication_time: entry["LastReplicationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                last_application_consistent_replication_time: entry
                    ["LastApplicationConsistentReplicationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                last_normal_replication_time: entry["LastNormalReplicationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                average_replication_latency: entry["AverageReplicationLatency"]
                    .as_u64()
                    .unwrap_or_default(),
                average_replication_size: entry["AverageReplicationSize"]
                    .as_u64()
                    .unwrap_or_default(),
                last_replication_size: entry["LastReplicationSize"].as_u64().unwrap_or_default(),
                maximum_replication_size: entry["MaximumReplicationSize"]
                    .as_u64()
                    .unwrap_or_default(),
                pending_replication_size: entry["PendingReplicationSize"]
                    .as_u64()
                    .unwrap_or_default(),
                successful_replication_count: entry["SuccessfulReplicationCount"]
                    .as_u64()
                    .unwrap_or_default(),
                failed_replication_count: entry["FailedReplicationCount"]
                    .as_u64()
                    .unwrap_or_default(),
                missed_replication_count: entry["MissedReplicationCount"]
                    .as_u64()
                    .unwrap_or_default(),
            });
        }

        Ok(MeasureVmReplicationOutput {
            replication_health: output,
        })
    }
}

register_tool!(MeasureVmReplicationTool);
