use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmMigrationNetworkInput {
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MigrationNetworkInfo {
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub subnet: String,
    pub priority: u32,
    pub metric: u32,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmMigrationNetworkOutput {
    pub networks: Vec<MigrationNetworkInfo>,
}

#[derive(Default)]
pub struct GetVmMigrationNetworkTool;

#[async_trait]
impl HyperVTool for GetVmMigrationNetworkTool {
    const NAME: &'static str = "hyperv_get_vm_migration_network";
    const DESCRIPTION: &'static str =
        "Gets the networks added for migration to one or more virtual machine hosts.";
    type Input = GetVmMigrationNetworkInput;
    type Output = GetVmMigrationNetworkOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMMigrationNetwork".to_string()];
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

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

        let mut output = Vec::with_capacity(networks.len());
        for network in networks {
            output.push(MigrationNetworkInfo {
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

        Ok(GetVmMigrationNetworkOutput { networks: output })
    }
}

register_tool!(GetVmMigrationNetworkTool);
