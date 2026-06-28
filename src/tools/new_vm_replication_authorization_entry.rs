use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewVmReplicationAuthorizationEntryInput {
    /// Server that is allowed to send replication data to the Replica server.
    /// Fully-qualified domain names are supported; wildcards may be used in the
    /// first octet (for example, *.contoso.com).
    #[serde(rename = "allowedPrimaryServer")]
    pub allowed_primary_server: String,
    /// Location to store the Replica virtual hard disk files sent from the
    /// allowed server when a new Replica virtual machine is created.
    #[serde(rename = "replicaStorageLocation")]
    pub replica_storage_location: String,
    /// Trust group that the allowed primary server belongs to.
    pub trust_group: String,
    /// Hyper-V host on which the authorization entry is to be created.
    /// Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
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
pub struct NewVmReplicationAuthorizationEntryOutput {
    pub entries: Vec<VmReplicationAuthorizationEntryInfo>,
}

#[derive(Default)]
pub struct NewVmReplicationAuthorizationEntryTool;

#[async_trait]
impl HyperVTool for NewVmReplicationAuthorizationEntryTool {
    const NAME: &'static str = "hyperv_new_vm_replication_authorization_entry";
    const DESCRIPTION: &'static str =
        "Creates a new authorization entry that allows one or more primary servers to replicate data to a specified Replica server.";
    type Input = NewVmReplicationAuthorizationEntryInput;
    type Output = NewVmReplicationAuthorizationEntryOutput;

    async fn run(
        &self,
        ctx: &ToolContext,
        input: Self::Input,
    ) -> Result<Self::Output, ToolError> {
        if input.allowed_primary_server.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "AllowedPrimaryServer must not be empty".to_string(),
            ));
        }
        if input.replica_storage_location.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "ReplicaStorageLocation must not be empty".to_string(),
            ));
        }
        if input.trust_group.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "TrustGroup must not be empty".to_string(),
            ));
        }

        let mut args = vec!["New-VMReplicationAuthorizationEntry".to_string()];
        args.push(format!(
            "-AllowedPrimaryServer '{}'",
            escape_ps_string(&input.allowed_primary_server)
        ));
        args.push(format!(
            "-ReplicaStorageLocation '{}'",
            escape_ps_string(&input.replica_storage_location)
        ));
        args.push(format!("-TrustGroup '{}'", escape_ps_string(&input.trust_group)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
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
                trust_group: entry["TrustGroup"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(NewVmReplicationAuthorizationEntryOutput { entries: output })
    }
}

register_tool!(NewVmReplicationAuthorizationEntryTool);
