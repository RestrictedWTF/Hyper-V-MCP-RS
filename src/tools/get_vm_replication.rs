use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmReplicationInput {
    /// Name of the virtual machine whose replication settings are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the replication relationship to retrieve. If omitted, all matching relationships are returned.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Display name of the primary server whose replicated virtual machines are to be retrieved.
    #[serde(default, rename = "primaryServerName")]
    pub primary_server_name: Option<String>,
    /// Display name of the Replica server whose replicated virtual machines are to be retrieved.
    #[serde(default, rename = "replicaServerName")]
    pub replica_server_name: Option<String>,
    /// Type of the replication relationship. Valid values are "Simple", "Extended", or "Test".
    #[serde(default, rename = "replicationRelationshipType")]
    pub replication_relationship_type: Option<String>,
    /// Port number of the Replica server.
    #[serde(default, rename = "replicaServerPort")]
    pub replica_server_port: Option<i32>,
    /// Authentication type of the replication relationship. Valid values are "Kerberos" or "Certificate".
    #[serde(default, rename = "authenticationType")]
    pub authentication_type: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationInfo {
    pub name: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "replicaServerDisplayName")]
    pub replica_server_display_name: String,
    pub state: String,
    pub health: String,
    #[serde(rename = "lastReplicationTime")]
    pub last_replication_time: String,
    #[serde(rename = "lastApplyTime")]
    pub last_apply_time: Option<String>,
    #[serde(rename = "frequencySec")]
    pub frequency_sec: u32,
    #[serde(rename = "authType")]
    pub auth_type: String,
    #[serde(rename = "applicationConsistentSnapshotFrequency")]
    pub application_consistent_snapshot_frequency: u32,
    pub compression: String,
    #[serde(rename = "backupExclusions")]
    pub backup_exclusions: Vec<String>,
    #[serde(rename = "replicatedDisks")]
    pub replicated_disks: Vec<String>,
    #[serde(rename = "pendingReplicationSize")]
    pub pending_replication_size: String,
    #[serde(rename = "failedOver")]
    pub failed_over: bool,
    #[serde(rename = "autoResynchronizeEnabled")]
    pub auto_resynchronize_enabled: bool,
    #[serde(rename = "autoResynchronizeIntervalStart")]
    pub auto_resynchronize_interval_start: Option<String>,
    #[serde(rename = "autoResynchronizeIntervalEnd")]
    pub auto_resynchronize_interval_end: Option<String>,
    #[serde(rename = "recoveryHistory")]
    pub recovery_history: u32,
    #[serde(rename = "replicationMode")]
    pub replication_mode: String,
    #[serde(rename = "reverseReplicationServer")]
    pub reverse_replication_server: String,
    #[serde(rename = "testReplicaSystem")]
    pub test_replica_system: String,
    #[serde(rename = "testFailoverReplicaSystem")]
    pub test_failover_replica_system: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmReplicationOutput {
    pub replications: Vec<VmReplicationInfo>,
}

#[derive(Default)]
pub struct GetVmReplicationTool;

fn strings_from(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect(),
        serde_json::Value::String(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

#[async_trait]
impl HyperVTool for GetVmReplicationTool {
    const NAME: &'static str = "hyperv_get_vm_replication";
    const DESCRIPTION: &'static str = "Gets the replication settings for a virtual machine.";
    type Input = GetVmReplicationInput;
    type Output = GetVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMReplication".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Replication name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(primary) = &input.primary_server_name {
            if primary.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Primary server name must not be empty".to_string(),
                ));
            }
            args.push(format!("-PrimaryServerName '{}'", escape_ps_string(primary)));
        }
        if let Some(replica) = &input.replica_server_name {
            if replica.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Replica server name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ReplicaServerName '{}'", escape_ps_string(replica)));
        }
        if let Some(rel_type) = &input.replication_relationship_type {
            if rel_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Replication relationship type must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-ReplicationRelationshipType '{}'",
                escape_ps_string(rel_type)
            ));
        }
        if let Some(port) = input.replica_server_port {
            args.push(format!("-ReplicaServerPort {}", port));
        }
        if let Some(auth) = &input.authentication_type {
            if auth.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Authentication type must not be empty".to_string(),
                ));
            }
            args.push(format!("-AuthenticationType '{}'", escape_ps_string(auth)));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, VMName, VMId, ComputerName, ReplicaServerDisplayName, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}}, \
             @{{N='LastReplicationTime';E={{$_.LastReplicationTime.ToString()}}}}, \
             @{{N='LastApplyTime';E={{$_.LastApplyTime.ToString()}}}}, \
             FrequencySec, \
             @{{N='AuthType';E={{$_.AuthType.ToString()}}}}, \
             ApplicationConsistentSnapshotFrequency, \
             @{{N='Compression';E={{$_.Compression.ToString()}}}}, \
             BackupExclusions, ReplicatedDisks, \
             @{{N='PendingReplicationSize';E={{$_.PendingReplicationSize.ToString()}}}}, \
             FailedOver, AutoResynchronizeEnabled, \
             @{{N='AutoResynchronizeIntervalStart';E={{$_.AutoResynchronizeIntervalStart.ToString()}}}}, \
             @{{N='AutoResynchronizeIntervalEnd';E={{$_.AutoResynchronizeIntervalEnd.ToString()}}}}, \
             RecoveryHistory, \
             @{{N='ReplicationMode';E={{$_.ReplicationMode.ToString()}}}}, \
             ReverseReplicationServer, TestReplicaSystem, TestFailoverReplicaSystem | \
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

        let replications = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(replications.len());
        for rep in replications {
            output.push(VmReplicationInfo {
                name: rep["Name"].as_str().unwrap_or_default().to_string(),
                vm_name: rep["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: rep["VMId"].as_str().unwrap_or_default().to_string(),
                computer_name: rep["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_display_name: rep["ReplicaServerDisplayName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                state: rep["State"].as_str().unwrap_or_default().to_string(),
                health: rep["Health"].as_str().unwrap_or_default().to_string(),
                last_replication_time: rep["LastReplicationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                last_apply_time: rep["LastApplyTime"].as_str().map(String::from),
                frequency_sec: rep["FrequencySec"].as_u64().unwrap_or_default() as u32,
                auth_type: rep["AuthType"].as_str().unwrap_or_default().to_string(),
                application_consistent_snapshot_frequency: rep
                    ["ApplicationConsistentSnapshotFrequency"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                compression: rep["Compression"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                backup_exclusions: strings_from(&rep["BackupExclusions"]),
                replicated_disks: strings_from(&rep["ReplicatedDisks"]),
                pending_replication_size: rep["PendingReplicationSize"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                failed_over: rep["FailedOver"].as_bool().unwrap_or_default(),
                auto_resynchronize_enabled: rep["AutoResynchronizeEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
                auto_resynchronize_interval_start: rep["AutoResynchronizeIntervalStart"]
                    .as_str()
                    .map(String::from),
                auto_resynchronize_interval_end: rep["AutoResynchronizeIntervalEnd"]
                    .as_str()
                    .map(String::from),
                recovery_history: rep["RecoveryHistory"].as_u64().unwrap_or_default() as u32,
                replication_mode: rep["ReplicationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                reverse_replication_server: rep["ReverseReplicationServer"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                test_replica_system: rep["TestReplicaSystem"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                test_failover_replica_system: rep["TestFailoverReplicaSystem"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmReplicationOutput {
            replications: output,
        })
    }
}

register_tool!(GetVmReplicationTool);
