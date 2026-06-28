use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmReplicationAuthorizationEntryInput {
    /// Allowed primary server for which to retrieve authorization entries.
    #[serde(default, rename = "allowedPrimaryServer")]
    pub allowed_primary_server: Option<String>,
    /// Replica storage location used to filter the authorization entries.
    #[serde(default, rename = "replicaStorageLocation")]
    pub replica_storage_location: Option<String>,
    /// Trust group used to filter the authorization entries.
    #[serde(default, rename = "trustGroup")]
    pub trust_group: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationAuthorizationEntryInfo {
    pub allowed_primary_server: String,
    pub replica_storage_location: String,
    pub trust_group: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmReplicationAuthorizationEntryOutput {
    pub entries: Vec<VmReplicationAuthorizationEntryInfo>,
}

#[derive(Default)]
pub struct GetVmReplicationAuthorizationEntryTool;

#[async_trait]
impl HyperVTool for GetVmReplicationAuthorizationEntryTool {
    const NAME: &'static str = "hyperv_get_vm_replication_authorization_entry";
    const DESCRIPTION: &'static str = "Gets the authorization entries of a Replica server.";
    type Input = GetVmReplicationAuthorizationEntryInput;
    type Output = GetVmReplicationAuthorizationEntryOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMReplicationAuthorizationEntry".to_string()];

        if let Some(server) = &input.allowed_primary_server {
            if server.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Allowed primary server must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-AllowedPrimaryServer '{}'",
                escape_ps_string(server)
            ));
        }

        if let Some(location) = &input.replica_storage_location {
            if location.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Replica storage location must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-ReplicaStorageLocation '{}'",
                escape_ps_string(location)
            ));
        }

        if let Some(group) = &input.trust_group {
            if group.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Trust group must not be empty".to_string(),
                ));
            }
            args.push(format!("-TrustGroup '{}'", escape_ps_string(group)));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

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

        Ok(GetVmReplicationAuthorizationEntryOutput { entries: output })
    }
}

register_tool!(GetVmReplicationAuthorizationEntryTool);
