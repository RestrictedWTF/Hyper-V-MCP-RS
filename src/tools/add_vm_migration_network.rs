use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmMigrationNetworkInput {
    /// Subnet string (IPv4 or IPv6) identifying the migration network to add.
    pub subnet: String,
    /// Priority of the migration network. Multiple networks can share the same priority.
    #[serde(default)]
    pub priority: Option<u32>,
    /// Hyper-V host on which to add the migration network. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VMMigrationNetworkInfo {
    pub subnet: String,
    pub priority: Option<u32>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmMigrationNetworkOutput {
    pub networks: Vec<VMMigrationNetworkInfo>,
}

#[derive(Default)]
pub struct AddVmMigrationNetworkTool;

#[async_trait]
impl HyperVTool for AddVmMigrationNetworkTool {
    const NAME: &'static str = "hyperv_add_vm_migration_network";
    const DESCRIPTION: &'static str =
        "Adds a network for virtual machine migration on one or more virtual machine hosts.";
    type Input = AddVmMigrationNetworkInput;
    type Output = AddVmMigrationNetworkOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.subnet.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Subnet must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Add-VMMigrationNetwork -Subnet '{}'",
            escape_ps_string(&input.subnet)
        )];

        if let Some(priority) = input.priority {
            args.push(format!("-Priority {}", priority));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Subnet, Priority, ComputerName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
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
            output.push(VMMigrationNetworkInfo {
                subnet: network["Subnet"].as_str().unwrap_or_default().to_string(),
                priority: network["Priority"].as_u64().map(|v| v as u32),
                computer_name: network["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: network["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(AddVmMigrationNetworkOutput { networks: output })
    }
}

register_tool!(AddVmMigrationNetworkTool);
