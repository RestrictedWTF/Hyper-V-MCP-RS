use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnableVmReplicationInput {
    /// Name of the virtual machine to configure for replication.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name or IP address of the Replica server.
    #[serde(rename = "replicaServerName")]
    pub replica_server_name: String,
    /// Port on the Replica server used for replication traffic.
    #[serde(rename = "replicaServerPort")]
    pub replica_server_port: u32,
    /// Authentication type to use: Kerberos or Certificate.
    #[serde(rename = "authenticationType")]
    pub authentication_type: String,
    /// Certificate thumbprint to use when AuthenticationType is Certificate.
    #[serde(default, rename = "certificateThumbprint")]
    pub certificate_thumbprint: Option<String>,
    /// Compress replication data sent over the network.
    #[serde(default, rename = "compressionEnabled")]
    pub compression_enabled: Option<bool>,
    /// Number of additional recovery points to store on the replica.
    #[serde(default, rename = "recoveryHistory")]
    pub recovery_history: Option<i32>,
    /// Frequency, in seconds, at which changes are replicated.
    #[serde(default, rename = "replicationFrequencySec")]
    pub replication_frequency_sec: Option<i32>,
    /// Bypass a proxy server when replicating data.
    #[serde(default, rename = "bypassProxyServer")]
    pub bypass_proxy_server: Option<bool>,
    /// Hyper-V host that has the virtual machine. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationInfo {
    pub name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "replicationState")]
    pub replication_state: String,
    #[serde(rename = "replicationMode")]
    pub replication_mode: String,
    #[serde(rename = "primaryServer")]
    pub primary_server: String,
    #[serde(rename = "replicaServer")]
    pub replica_server: String,
    #[serde(rename = "replicaServerPort")]
    pub replica_server_port: u32,
    #[serde(rename = "authenticationType")]
    pub authentication_type: String,
    pub health: String,
    #[serde(rename = "lastReplicationTime")]
    pub last_replication_time: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnableVmReplicationOutput {
    pub replications: Vec<VmReplicationInfo>,
}

#[derive(Default)]
pub struct EnableVmReplicationTool;

#[async_trait]
impl HyperVTool for EnableVmReplicationTool {
    const NAME: &'static str = "hyperv_enable_vm_replication";
    const DESCRIPTION: &'static str = "Enables replication of a virtual machine.";
    type Input = EnableVmReplicationInput;
    type Output = EnableVmReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.replica_server_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Replica server name must not be empty".to_string(),
            ));
        }
        if input.replica_server_port == 0 || input.replica_server_port > 65535 {
            return Err(ToolError::InvalidInput(
                "Replica server port must be between 1 and 65535".to_string(),
            ));
        }
        let auth = input.authentication_type.trim();
        if auth.is_empty() {
            return Err(ToolError::InvalidInput(
                "Authentication type must not be empty".to_string(),
            ));
        }
        if auth.eq_ignore_ascii_case("Certificate") {
            match &input.certificate_thumbprint {
                None => {
                    return Err(ToolError::InvalidInput(
                        "Certificate thumbprint is required when authentication type is Certificate"
                            .to_string(),
                    ));
                }
                Some(thumb) if thumb.trim().is_empty() => {
                    return Err(ToolError::InvalidInput(
                        "Certificate thumbprint must not be empty when authentication type is Certificate"
                            .to_string(),
                    ));
                }
                _ => {}
            }
        }

        let mut args = vec![format!(
            "Enable-VMReplication -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];
        args.push(format!(
            "-ReplicaServerName '{}'",
            escape_ps_string(&input.replica_server_name)
        ));
        args.push(format!("-ReplicaServerPort {}", input.replica_server_port));
        args.push(format!("-AuthenticationType '{}'", escape_ps_string(auth)));

        if let Some(thumb) = &input.certificate_thumbprint {
            args.push(format!(
                "-CertificateThumbprint '{}'",
                escape_ps_string(thumb)
            ));
        }
        if let Some(enabled) = input.compression_enabled {
            args.push(format!("-CompressionEnabled ${}", enabled));
        }
        if let Some(history) = input.recovery_history {
            args.push(format!("-RecoveryHistory {}", history));
        }
        if let Some(freq) = input.replication_frequency_sec {
            args.push(format!("-ReplicationFrequencySec {}", freq));
        }
        if let Some(bypass) = input.bypass_proxy_server {
            args.push(format!("-BypassProxyServer ${}", bypass));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }


        let ps = format!(
            "{} | Select-Object Name, VMId, \
             @{{N='ReplicationState';E={{$_.ReplicationState.ToString()}}}}, \
             @{{N='ReplicationMode';E={{$_.ReplicationMode.ToString()}}}}, \
             PrimaryServer, ReplicaServer, ReplicaServerPort, \
             @{{N='AuthenticationType';E={{$_.AuthenticationType.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}}, \
             @{{N='LastReplicationTime';E={{$_.LastReplicationTime.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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
            output.push(VmReplicationInfo {
                name: replication["Name"].as_str().unwrap_or_default().to_string(),
                vm_id: replication["VMId"].as_str().unwrap_or_default().to_string(),
                replication_state: replication["ReplicationState"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_mode: replication["ReplicationMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                primary_server: replication["PrimaryServer"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server: replication["ReplicaServer"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_port: replication["ReplicaServerPort"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                authentication_type: replication["AuthenticationType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                health: replication["Health"].as_str().unwrap_or_default().to_string(),
                last_replication_time: replication["LastReplicationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(EnableVmReplicationOutput {
            replications: output,
        })
    }
}

register_tool!(EnableVmReplicationTool);
