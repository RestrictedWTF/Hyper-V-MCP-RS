use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmReplicationInput {
    /// Name of the virtual machine whose replication settings are to be modified.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Authentication type used for replication: Kerberos or Certificate.
    #[serde(default, rename = "authenticationType")]
    pub authentication_type: Option<String>,
    /// Number of hours of recovery history to maintain.
    #[serde(default, rename = "recoveryHistory")]
    pub recovery_history: Option<u32>,
    /// Replication frequency in seconds. Valid values depend on the host version.
    #[serde(default, rename = "replicationFrequencySec")]
    pub replication_frequency_sec: Option<u32>,
    /// Frequency, in hours, at which VSS performs an application-consistent snapshot.
    #[serde(default, rename = "vssSnapshotFrequencyHour")]
    pub vss_snapshot_frequency_hour: Option<u32>,
    /// Enables or disables compression of replicated data.
    #[serde(default, rename = "compressionEnabled")]
    pub compression_enabled: Option<bool>,
    /// Enables or disables bypassing the proxy server for replication.
    #[serde(default, rename = "bypassProxyServer")]
    pub bypass_proxy_server: Option<bool>,
    /// Name of the Replica server.
    #[serde(default, rename = "replicaServerName")]
    pub replica_server_name: Option<String>,
    /// Port used on the Replica server.
    #[serde(default, rename = "replicaServerPort")]
    pub replica_server_port: Option<u32>,
    /// Thumbprint of the certificate used for certificate-based authentication.
    #[serde(default, rename = "certificateThumbprint")]
    pub certificate_thumbprint: Option<String>,
    /// Enables or disables automatic resynchronization.
    #[serde(default, rename = "autoResynchronizeEnabled")]
    pub auto_resynchronize_enabled: Option<bool>,
    /// Start of the automatic resynchronization window (TimeSpan string, e.g. "20:00:00").
    #[serde(default, rename = "autoResynchronizeIntervalStart")]
    pub auto_resynchronize_interval_start: Option<String>,
    /// End of the automatic resynchronization window (TimeSpan string, e.g. "23:59:00").
    #[serde(default, rename = "autoResynchronizeIntervalEnd")]
    pub auto_resynchronize_interval_end: Option<String>,
    /// Reverses replication direction for the virtual machine.
    #[serde(default)]
    pub reverse: Option<bool>,
    /// Primary server allowed to replicate to this Replica virtual machine.
    #[serde(default, rename = "allowedPrimaryServer")]
    pub allowed_primary_server: Option<String>,
    /// Configures the virtual machine as a Replica virtual machine.
    #[serde(default, rename = "asReplica")]
    pub as_replica: Option<bool>,
    /// Specifies that the virtual machine is restored from a backup.
    #[serde(default, rename = "useBackup")]
    pub use_backup: Option<bool>,
    /// Disables application-consistent snapshot replication.
    #[serde(default, rename = "disableVssSnapshotReplication")]
    pub disable_vss_snapshot_replication: Option<bool>,
    /// Enables or disables write order preservation across replicated disks.
    #[serde(default, rename = "enableWriteOrderPreservationAcrossDisks")]
    pub enable_write_order_preservation_across_disks: Option<bool>,
    /// Enables or disables replication of host-only KVP items.
    #[serde(default, rename = "replicateHostKvpItems")]
    pub replicate_host_kvp_items: Option<bool>,
    /// Scheduled start time for initial replication (DateTime string).
    #[serde(default, rename = "initialReplicationStartTime")]
    pub initial_replication_start_time: Option<String>,
    /// Paths of virtual hard disks to include in replication.
    #[serde(default, rename = "replicatedDiskPaths")]
    pub replicated_disk_paths: Option<Vec<String>>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationInfo {
    pub vm_name: String,
    pub computer_name: String,
    pub state: String,
    pub health: String,
    pub mode: String,
    pub authentication_type: String,
    pub replication_frequency_sec: u32,
    pub primary_server_name: String,
    pub replica_server_name: String,
    pub replica_server_port: u32,
    pub last_replication_time: String,
    pub last_test_failover_initiated_time: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmReplicationOutput {
    pub replication: Vec<VmReplicationInfo>,
}

#[derive(Default)]
pub struct SetVmReplicationTool;

#[async_trait]
impl HyperVTool for SetVmReplicationTool {
    const NAME: &'static str = "hyperv_set_vm_replication";
    const DESCRIPTION: &'static str = "Modifies the replication settings of a virtual machine.";
    type Input = SetVmReplicationInput;
    type Output = SetVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMReplication -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(server) = &input.replica_server_name {
            args.push(format!("-ReplicaServerName '{}'", escape_ps_string(server)));
        }
        if let Some(port) = input.replica_server_port {
            args.push(format!("-ReplicaServerPort {}", port));
        }
        if let Some(auth) = &input.authentication_type {
            args.push(format!("-AuthenticationType '{}'", escape_ps_string(auth)));
        }
        if let Some(thumbprint) = &input.certificate_thumbprint {
            args.push(format!(
                "-CertificateThumbprint '{}'",
                escape_ps_string(thumbprint)
            ));
        }
        if let Some(enabled) = input.compression_enabled {
            args.push(format!("-CompressionEnabled ${}", enabled));
        }
        if let Some(enabled) = input.replicate_host_kvp_items {
            args.push(format!("-ReplicateHostKvpItems ${}", enabled));
        }
        if let Some(bypass) = input.bypass_proxy_server {
            args.push(format!("-BypassProxyServer ${}", bypass));
        }
        if let Some(enabled) = input.enable_write_order_preservation_across_disks {
            args.push(format!(
                "-EnableWriteOrderPreservationAcrossDisks ${}",
                enabled
            ));
        }
        if let Some(time) = &input.initial_replication_start_time {
            args.push(format!(
                "-InitialReplicationStartTime '{}'",
                escape_ps_string(time)
            ));
        }
        if input.disable_vss_snapshot_replication == Some(true) {
            args.push("-DisableVSSSnapshotReplication".to_string());
        }
        if let Some(freq) = input.vss_snapshot_frequency_hour {
            args.push(format!("-VSSSnapshotFrequencyHour {}", freq));
        }
        if let Some(history) = input.recovery_history {
            args.push(format!("-RecoveryHistory {}", history));
        }
        if let Some(freq) = input.replication_frequency_sec {
            args.push(format!("-ReplicationFrequencySec {}", freq));
        }
        if let Some(paths) = &input.replicated_disk_paths {
            let escaped: Vec<String> = paths
                .iter()
                .map(|p| format!("'{}'", escape_ps_string(p)))
                .collect();
            args.push(format!("-ReplicatedDiskPaths @({})", escaped.join(",")));
        }
        if input.reverse == Some(true) {
            args.push("-Reverse".to_string());
        }
        if let Some(enabled) = input.auto_resynchronize_enabled {
            args.push(format!("-AutoResynchronizeEnabled ${}", enabled));
        }
        if let Some(start) = &input.auto_resynchronize_interval_start {
            args.push(format!(
                "-AutoResynchronizeIntervalStart '{}'",
                escape_ps_string(start)
            ));
        }
        if let Some(end) = &input.auto_resynchronize_interval_end {
            args.push(format!(
                "-AutoResynchronizeIntervalEnd '{}'",
                escape_ps_string(end)
            ));
        }
        if input.as_replica == Some(true) {
            args.push("-AsReplica".to_string());
        }
        if let Some(server) = &input.allowed_primary_server {
            args.push(format!(
                "-AllowedPrimaryServer '{}'",
                escape_ps_string(server)
            ));
        }
        if input.use_backup == Some(true) {
            args.push("-UseBackup".to_string());
        }

        let ps = format!(
            "{} | Select-Object \
             VMName, ComputerName, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}}, \
             @{{N='Mode';E={{$_.Mode.ToString()}}}}, \
             @{{N='AuthenticationType';E={{$_.AuthenticationType.ToString()}}}}, \
             ReplicationFrequencySec, PrimaryServerName, ReplicaServerName, ReplicaServerPort, \
             @{{N='LastReplicationTime';E={{$_.LastReplicationTime.ToString()}}}}, \
             @{{N='LastTestFailoverInitiatedTime';E={{$_.LastTestFailoverInitiatedTime.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            output.push(VmReplicationInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                state: item["State"].as_str().unwrap_or_default().to_string(),
                health: item["Health"].as_str().unwrap_or_default().to_string(),
                mode: item["Mode"].as_str().unwrap_or_default().to_string(),
                authentication_type: item["AuthenticationType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_frequency_sec: item["ReplicationFrequencySec"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                primary_server_name: item["PrimaryServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_name: item["ReplicaServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_port: item["ReplicaServerPort"].as_u64().unwrap_or_default() as u32,
                last_replication_time: item["LastReplicationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                last_test_failover_initiated_time: item["LastTestFailoverInitiatedTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmReplicationOutput {
            replication: output,
        })
    }
}

register_tool!(SetVmReplicationTool);
