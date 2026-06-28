use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmMigrationNetworkInput {
    /// Subnet of the migration network to configure (e.g. "192.168.1.0").
    pub subnet: String,
    /// New subnet mask for the migration network (e.g. "255.255.255.0").
    #[serde(default, rename = "subnetMask")]
    pub subnet_mask: Option<String>,
    /// New priority for the migration network.
    #[serde(default)]
    pub priority: Option<i32>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MigrationNetworkInfo {
    pub subnet: String,
    pub subnet_mask: String,
    pub priority: i32,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmMigrationNetworkOutput {
    pub networks: Vec<MigrationNetworkInfo>,
}

#[derive(Default)]
pub struct SetVmMigrationNetworkTool;

#[async_trait]
impl HyperVTool for SetVmMigrationNetworkTool {
    const NAME: &'static str = "hyperv_set_vm_migration_network";
    const DESCRIPTION: &'static str =
        "Sets the subnet, subnet mask, and/or priority of a migration network.";
    type Input = SetVmMigrationNetworkInput;
    type Output = SetVmMigrationNetworkOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.subnet.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Subnet must not be empty".to_string(),
            ));
        }

        if input.subnet_mask.is_none() && input.priority.is_none() {
            return Err(ToolError::InvalidInput(
                "At least one of subnet_mask or priority must be provided".to_string(),
            ));
        }

        if let Some(mask) = &input.subnet_mask {
            if mask.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SubnetMask must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec!["Set-VMMigrationNetwork".to_string()];
        args.push(format!("-Subnet '{}'", escape_ps_string(&input.subnet)));

        if let Some(mask) = &input.subnet_mask {
            args.push(format!("-SubnetMask '{}'", escape_ps_string(mask)));
        }
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
            "{} | Select-Object Subnet, SubnetMask, Priority, ComputerName | ConvertTo-Json -Compress -Depth 3",
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
                subnet: network["Subnet"].as_str().unwrap_or_default().to_string(),
                subnet_mask: network["SubnetMask"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                priority: network["Priority"].as_i64().unwrap_or_default() as i32,
                computer_name: network["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmMigrationNetworkOutput { networks: output })
    }
}

register_tool!(SetVmMigrationNetworkTool);
