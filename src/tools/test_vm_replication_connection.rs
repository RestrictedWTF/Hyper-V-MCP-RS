use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestVmReplicationConnectionInput {
    /// Name of the Replica server to test connectivity to.
    #[serde(rename = "replicaServerName")]
    pub replica_server_name: String,
    /// Port on the Replica server to use for the connection test.
    #[serde(rename = "replicaServerPort")]
    pub replica_server_port: u16,
    /// Authentication type to use. Valid values are Kerberos and Certificate.
    #[serde(rename = "authenticationType")]
    pub authentication_type: String,
    /// Certificate thumbprint to use when authenticationType is Certificate.
    #[serde(default, rename = "certificateThumbprint")]
    pub certificate_thumbprint: Option<String>,
    /// Hyper-V host to test from. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TestVmReplicationConnectionOutput {
    /// True if the connection to the Replica server was established successfully; otherwise false.
    pub success: bool,
}

#[derive(Default)]
pub struct TestVmReplicationConnectionTool;

#[async_trait]
impl HyperVTool for TestVmReplicationConnectionTool {
    const NAME: &'static str = "hyperv_test_vm_replication_connection";
    const DESCRIPTION: &'static str =
        "Tests the connection between a primary server and a Replica server.";
    type Input = TestVmReplicationConnectionInput;
    type Output = TestVmReplicationConnectionOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.replica_server_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "replica_server_name is required".to_string(),
            ));
        }
        if input.authentication_type.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "authentication_type is required".to_string(),
            ));
        }

        let mut args = vec!["Test-VMReplicationConnection".to_string()];
        args.push(format!(
            "-ReplicaServerName '{}'",
            escape_ps_string(&input.replica_server_name)
        ));
        args.push(format!("-ReplicaServerPort {}", input.replica_server_port));
        args.push(format!(
            "-AuthenticationType '{}'",
            escape_ps_string(&input.authentication_type)
        ));

        if let Some(thumbprint) = &input.certificate_thumbprint {
            args.push(format!(
                "-CertificateThumbprint '{}'",
                escape_ps_string(thumbprint)
            ));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!("{} | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let success = match raw {
            serde_json::Value::Bool(b) => b,
            serde_json::Value::Array(arr) if arr.is_empty() => {
                return Err(ToolError::Sidecar(
                    "empty response from sidecar".to_string(),
                ));
            }
            _ => {
                return Err(ToolError::Sidecar(format!(
                    "unexpected Test-VMReplicationConnection response: {}",
                    raw
                )));
            }
        };

        Ok(TestVmReplicationConnectionOutput { success })
    }
}

register_tool!(TestVmReplicationConnectionTool);
