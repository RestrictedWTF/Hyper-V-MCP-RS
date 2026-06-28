use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmReplicationServerInput {
    /// Hyper-V host to configure as a Replica server. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Specifies whether the host is enabled as a Replica server.
    #[serde(default, rename = "replicationEnabled")]
    pub replication_enabled: Option<bool>,
    /// Authentication types the Replica server will use: Kerberos, Certificate, or CertificateAndKerberos.
    #[serde(default, rename = "allowedAuthenticationType")]
    pub allowed_authentication_type: Option<String>,
    /// Specifies whether to accept replication requests from any server. When true,
    /// DefaultStorageLocation must also be specified.
    #[serde(default, rename = "replicationAllowedFromAnyServer")]
    pub replication_allowed_from_any_server: Option<bool>,
    /// Certificate thumbprint for mutual authentication. Required when the authentication type is Certificate.
    #[serde(default, rename = "certificateThumbprint")]
    pub certificate_thumbprint: Option<String>,
    /// Default location to store replica virtual hard disk files.
    #[serde(default, rename = "defaultStorageLocation")]
    pub default_storage_location: Option<String>,
    /// Port that the HTTP listener uses on the Replica server host for Kerberos authentication.
    #[serde(default, rename = "kerberosAuthenticationPort")]
    pub kerberos_authentication_port: Option<i32>,
    /// Port on which the Replica server will receive replication data using certificate-based authentication.
    #[serde(default, rename = "certificateAuthenticationPort")]
    pub certificate_authentication_port: Option<i32>,
    /// Monitoring interval as a TimeSpan string (e.g. "12:00:00").
    #[serde(default, rename = "monitoringInterval")]
    pub monitoring_interval: Option<String>,
    /// Monitoring start time as a TimeSpan string (e.g. "17:00:00").
    #[serde(default, rename = "monitoringStartTime")]
    pub monitoring_start_time: Option<String>,
    /// Run the cmdlet without requiring confirmation.
    #[serde(default)]
    pub force: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationServerInfo {
    pub computer_name: String,
    pub replication_enabled: bool,
    pub allowed_authentication_type: String,
    pub replication_allowed_from_any_server: bool,
    pub default_storage_location: String,
    pub certificate_thumbprint: String,
    pub kerberos_authentication_port: i32,
    pub certificate_authentication_port: i32,
    pub monitoring_interval: String,
    pub monitoring_start_time: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmReplicationServerOutput {
    pub servers: Vec<VmReplicationServerInfo>,
}

#[derive(Default)]
pub struct SetVmReplicationServerTool;

#[async_trait]
impl HyperVTool for SetVmReplicationServerTool {
    const NAME: &'static str = "hyperv_set_vm_replication_server";
    const DESCRIPTION: &'static str = "Configures a host as a Replica server.";
    type Input = SetVmReplicationServerInput;
    type Output = SetVmReplicationServerOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.replication_enabled.is_none()
            && input.allowed_authentication_type.is_none()
            && input.replication_allowed_from_any_server.is_none()
            && input.certificate_thumbprint.is_none()
            && input.default_storage_location.is_none()
            && input.kerberos_authentication_port.is_none()
            && input.certificate_authentication_port.is_none()
            && input.monitoring_interval.is_none()
            && input.monitoring_start_time.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one Replica server setting must be provided".to_string(),
            ));
        }

        if input.replication_allowed_from_any_server == Some(true)
            && input.default_storage_location.is_none()
        {
            return Err(ToolError::InvalidInput(
                "DefaultStorageLocation must be provided when ReplicationAllowedFromAnyServer is true"
                    .to_string(),
            ));
        }

        let mut args = vec!["Set-VMReplicationServer".to_string()];

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(enabled) = input.replication_enabled {
            args.push(format!("-ReplicationEnabled ${}", enabled));
        }
        if let Some(auth) = &input.allowed_authentication_type {
            if auth.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "AllowedAuthenticationType must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-AllowedAuthenticationType '{}'",
                escape_ps_string(auth)
            ));
        }
        if let Some(allowed) = input.replication_allowed_from_any_server {
            args.push(format!("-ReplicationAllowedFromAnyServer ${}", allowed));
        }
        if let Some(thumbprint) = &input.certificate_thumbprint {
            if thumbprint.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "CertificateThumbprint must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-CertificateThumbprint '{}'",
                escape_ps_string(thumbprint)
            ));
        }
        if let Some(location) = &input.default_storage_location {
            if location.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "DefaultStorageLocation must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-DefaultStorageLocation '{}'",
                escape_ps_string(location)
            ));
        }
        if let Some(port) = input.kerberos_authentication_port {
            args.push(format!("-KerberosAuthenticationPort {}", port));
        }
        if let Some(port) = input.certificate_authentication_port {
            args.push(format!("-CertificateAuthenticationPort {}", port));
        }
        if let Some(interval) = &input.monitoring_interval {
            if interval.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "MonitoringInterval must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-MonitoringInterval '{}'",
                escape_ps_string(interval)
            ));
        }
        if let Some(start) = &input.monitoring_start_time {
            if start.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "MonitoringStartTime must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-MonitoringStartTime '{}'",
                escape_ps_string(start)
            ));
        }
        if input.force == Some(true) {
            args.push("-Force".to_string());
        }

        let ps = format!(
            "{} | Select-Object \
             ComputerName, ReplicationEnabled, \
             @{{N='AllowedAuthenticationType';E={{$_.AllowedAuthenticationType.ToString()}}}}, \
             ReplicationAllowedFromAnyServer, DefaultStorageLocation, CertificateThumbprint, \
             KerberosAuthenticationPort, CertificateAuthenticationPort, \
             @{{N='MonitoringInterval';E={{$_.MonitoringInterval.ToString()}}}}, \
             @{{N='MonitoringStartTime';E={{$_.MonitoringStartTime.ToString()}}}} | \
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

        let servers = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(servers.len());
        for server in servers {
            output.push(VmReplicationServerInfo {
                computer_name: server["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_enabled: server["ReplicationEnabled"].as_bool().unwrap_or_default(),
                allowed_authentication_type: server["AllowedAuthenticationType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_allowed_from_any_server: server["ReplicationAllowedFromAnyServer"]
                    .as_bool()
                    .unwrap_or_default(),
                default_storage_location: server["DefaultStorageLocation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                certificate_thumbprint: server["CertificateThumbprint"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                kerberos_authentication_port: server["KerberosAuthenticationPort"]
                    .as_i64()
                    .unwrap_or_default() as i32,
                certificate_authentication_port: server["CertificateAuthenticationPort"]
                    .as_i64()
                    .unwrap_or_default() as i32,
                monitoring_interval: server["MonitoringInterval"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                monitoring_start_time: server["MonitoringStartTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmReplicationServerOutput { servers: output })
    }
}

register_tool!(SetVmReplicationServerTool);
