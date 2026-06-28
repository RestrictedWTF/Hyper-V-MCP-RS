use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmReplicationAuthorizationEntryInput {
    /// The fully qualified domain name (FQDN) or NetBIOS name of the primary server whose
    /// authorization entry is to be modified.
    #[serde(rename = "allowedPrimaryServer")]
    pub allowed_primary_server: String,
    /// Hyper-V host on which to modify the authorization entry. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// New location for storing replication data on the Replica server.
    #[serde(default, rename = "replicaStorageLocation")]
    pub replica_storage_location: Option<String>,
    /// New trust group tag for the authorization entry.
    #[serde(default, rename = "trustGroup")]
    pub trust_group: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationAuthorizationEntryInfo {
    #[serde(rename = "allowedPrimaryServer")]
    pub allowed_primary_server: String,
    #[serde(rename = "replicaStorageLocation")]
    pub replica_storage_location: String,
    #[serde(rename = "trustGroup")]
    pub trust_group: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmReplicationAuthorizationEntryOutput {
    pub entries: Vec<VmReplicationAuthorizationEntryInfo>,
}

#[derive(Default)]
pub struct SetVmReplicationAuthorizationEntryTool;

#[async_trait]
impl HyperVTool for SetVmReplicationAuthorizationEntryTool {
    const NAME: &'static str = "hyperv_set_vm_replication_authorization_entry";
    const DESCRIPTION: &'static str = "Modifies an authorization entry on a Replica server.";
    type Input = SetVmReplicationAuthorizationEntryInput;
    type Output = SetVmReplicationAuthorizationEntryOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.allowed_primary_server.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "AllowedPrimaryServer must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Set-VMReplicationAuthorizationEntry".to_string()];
        args.push(format!(
            "-AllowedPrimaryServer '{}'",
            escape_ps_string(&input.allowed_primary_server)
        ));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(location) = &input.replica_storage_location {
            args.push(format!(
                "-ReplicaStorageLocation '{}'",
                escape_ps_string(location)
            ));
        }

        if let Some(group) = &input.trust_group {
            args.push(format!("-TrustGroup '{}'", escape_ps_string(group)));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object AllowedPrimaryServer, ReplicaStorageLocation, TrustGroup | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let entries = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(entries.len());
        for entry in entries {
            output.push(VmReplicationAuthorizationEntryInfo {
                allowed_primary_server: entry["AllowedPrimaryServer"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_storage_location: entry["ReplicaStorageLocation"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                trust_group: entry["TrustGroup"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SetVmReplicationAuthorizationEntryOutput { entries: output })
    }
}

register_tool!(SetVmReplicationAuthorizationEntryTool);
