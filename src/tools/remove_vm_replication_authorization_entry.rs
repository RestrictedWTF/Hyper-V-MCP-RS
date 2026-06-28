use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmReplicationAuthorizationEntryInput {
    /// Allowed primary server for which the authorization entry should be removed.
    #[serde(default, rename = "allowedPrimaryServer")]
    pub allowed_primary_server: Option<String>,
    /// Trust group for which the authorization entries should be removed.
    #[serde(default, rename = "trustGroup")]
    pub trust_group: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedAuthorizationEntryInfo {
    #[serde(rename = "allowedPrimaryServer")]
    pub allowed_primary_server: String,
    #[serde(rename = "replicaStorageLocation")]
    pub replica_storage_location: String,
    #[serde(rename = "trustGroup")]
    pub trust_group: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmReplicationAuthorizationEntryOutput {
    /// Authorization entries that were removed.
    pub removed: Vec<RemovedAuthorizationEntryInfo>,
}

#[derive(Default)]
pub struct RemoveVmReplicationAuthorizationEntryTool;

#[async_trait]
impl HyperVTool for RemoveVmReplicationAuthorizationEntryTool {
    const NAME: &'static str = "hyperv_remove_vm_replication_authorization_entry";
    const DESCRIPTION: &'static str = "Removes an authorization entry from a Replica server.";
    type Input = RemoveVmReplicationAuthorizationEntryInput;
    type Output = RemoveVmReplicationAuthorizationEntryOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let has_primary = input
            .allowed_primary_server
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_trust = input
            .trust_group
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_primary && !has_trust {
            return Err(ToolError::InvalidInput(
                "Either allowedPrimaryServer or trustGroup must be provided".to_string(),
            ));
        }
        if has_primary && has_trust {
            return Err(ToolError::InvalidInput(
                "allowedPrimaryServer and trustGroup cannot both be provided".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMReplicationAuthorizationEntry".to_string()];

        if has_primary {
            args.push(format!(
                "-AllowedPrimaryServer '{}'",
                escape_ps_string(input.allowed_primary_server.as_ref().unwrap())
            ));
        } else if has_trust {
            args.push(format!(
                "-TrustGroup '{}'",
                escape_ps_string(input.trust_group.as_ref().unwrap())
            ));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

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

        let mut removed = Vec::with_capacity(entries.len());
        for entry in entries {
            removed.push(RemovedAuthorizationEntryInfo {
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

        Ok(RemoveVmReplicationAuthorizationEntryOutput { removed })
    }
}

register_tool!(RemoveVmReplicationAuthorizationEntryTool);
