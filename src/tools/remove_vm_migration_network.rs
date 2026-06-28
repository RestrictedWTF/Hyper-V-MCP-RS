use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmMigrationNetworkInput {
    /// Subnet of the migration network to remove, in CIDR notation (e.g. 192.168.1.0/24).
    pub subnet: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedMigrationNetworkInfo {
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub subnet: String,
    pub priority: u32,
    pub metric: u32,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmMigrationNetworkOutput {
    /// Migration networks that were removed.
    pub removed: Vec<RemovedMigrationNetworkInfo>,
}

#[derive(Default)]
pub struct RemoveVmMigrationNetworkTool;

#[async_trait]
impl HyperVTool for RemoveVmMigrationNetworkTool {
    const NAME: &'static str = "hyperv_remove_vm_migration_network";
    const DESCRIPTION: &'static str = "Removes a network from use with migration.";
    type Input = RemoveVmMigrationNetworkInput;
    type Output = RemoveVmMigrationNetworkOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.subnet.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "subnet must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMMigrationNetwork".to_string()];
        args.push(format!("-Subnet '{}'", escape_ps_string(&input.subnet)));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             ComputerName, Subnet, Priority, Metric, IsDeleted | \
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

        let networks = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut removed = Vec::with_capacity(networks.len());
        for network in networks {
            removed.push(RemovedMigrationNetworkInfo {
                computer_name: network["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                subnet: network["Subnet"].as_str().unwrap_or_default().to_string(),
                priority: network["Priority"].as_u64().unwrap_or_default() as u32,
                metric: network["Metric"].as_u64().unwrap_or_default() as u32,
                is_deleted: network["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(RemoveVmMigrationNetworkOutput { removed })
    }
}

register_tool!(RemoveVmMigrationNetworkTool);
