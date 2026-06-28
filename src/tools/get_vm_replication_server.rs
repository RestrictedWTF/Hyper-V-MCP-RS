use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmReplicationServerInput {
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationServerInfo {
    pub computer_name: String,
    pub replication_enabled: bool,
    pub allowed_authentication_type: String,
    pub certificate_authentication_port: u32,
    pub kerberos_authentication_port: u32,
    pub replication_allowed_from_any_server: bool,
    pub default_storage_location: String,
    pub certificate_thumbprint: String,
    pub monitoring_interval: String,
    pub monitoring_start_time: String,
    pub operational_status: Vec<String>,
    pub status_descriptions: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmReplicationServerOutput {
    pub servers: Vec<VmReplicationServerInfo>,
}

#[derive(Default)]
pub struct GetVmReplicationServerTool;

#[async_trait]
impl HyperVTool for GetVmReplicationServerTool {
    const NAME: &'static str = "hyperv_get_vm_replication_server";
    const DESCRIPTION: &'static str =
        "Gets the replication and authentication settings of a Replica server.";
    type Input = GetVmReplicationServerInput;
    type Output = GetVmReplicationServerOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMReplicationServer".to_string()];
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let select_clause = "ComputerName, \
             ReplicationEnabled, \
             @{N='AllowedAuthenticationType';E={$_.AllowedAuthenticationType.ToString()}}, \
             CertificateAuthenticationPort, \
             KerberosAuthenticationPort, \
             ReplicationAllowedFromAnyServer, \
             DefaultStorageLocation, \
             CertificateThumbprint, \
             @{N='MonitoringInterval';E={$_.MonitoringInterval.ToString()}}, \
             @{N='MonitoringStartTime';E={$_.MonitoringStartTime.ToString()}}, \
             @{N='OperationalStatus';E={$_.OperationalStatus | ForEach-Object { $_.ToString() }}}, \
             StatusDescriptions";

        let ps = format!(
            "{} | Select-Object {} | ConvertTo-Json -Compress -Depth 3",
            args.join(" "),
            select_clause
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
            let operational_status: Vec<String> = match &server["OperationalStatus"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                serde_json::Value::String(s) => vec![s.clone()],
                _ => Vec::new(),
            };

            let status_descriptions: Vec<String> = match &server["StatusDescriptions"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                serde_json::Value::String(s) => vec![s.clone()],
                _ => Vec::new(),
            };

            output.push(VmReplicationServerInfo {
                computer_name: server["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replication_enabled: server["ReplicationEnabled"].as_bool().unwrap_or(false),
                allowed_authentication_type: server["AllowedAuthenticationType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                certificate_authentication_port: server["CertificateAuthenticationPort"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                kerberos_authentication_port: server["KerberosAuthenticationPort"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                replication_allowed_from_any_server: server["ReplicationAllowedFromAnyServer"]
                    .as_bool()
                    .unwrap_or(false),
                default_storage_location: server["DefaultStorageLocation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                certificate_thumbprint: server["CertificateThumbprint"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                monitoring_interval: server["MonitoringInterval"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                monitoring_start_time: server["MonitoringStartTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                operational_status,
                status_descriptions,
            });
        }

        Ok(GetVmReplicationServerOutput { servers: output })
    }
}

register_tool!(GetVmReplicationServerTool);
