use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EnableVmMigrationInput {
    /// Hyper-V host on which to enable migration. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnabledVmMigrationHostInfo {
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub virtual_machine_migration_enabled: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct EnableVmMigrationOutput {
    pub hosts: Vec<EnabledVmMigrationHostInfo>,
}

#[derive(Default)]
pub struct EnableVmMigrationTool;

#[async_trait]
impl HyperVTool for EnableVmMigrationTool {
    const NAME: &'static str = "hyperv_enable_vm_migration";
    const DESCRIPTION: &'static str =
        "Enables migration on one or more virtual machine hosts.";
    type Input = EnableVmMigrationInput;
    type Output = EnableVmMigrationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Enable-VMMigration".to_string()];
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             ComputerName, \
             VirtualMachineMigrationEnabled | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let hosts = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(hosts.len());
        for host in hosts {
            output.push(EnabledVmMigrationHostInfo {
                computer_name: host["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                virtual_machine_migration_enabled: host["VirtualMachineMigrationEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
            });
        }

        Ok(EnableVmMigrationOutput { hosts: output })
    }
}

register_tool!(EnableVmMigrationTool);
