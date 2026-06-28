use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostInput {
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostInfo {
    pub computer_name: String,
    pub logical_processor_count: u32,
    pub memory_capacity: String,
    pub virtual_machine_path: String,
    pub virtual_hard_disk_path: String,
    pub maximum_storage_migrations: u32,
    pub maximum_virtual_machine_migrations: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostOutput {
    pub hosts: Vec<VmHostInfo>,
}

#[derive(Default)]
pub struct GetVmHostTool;

#[async_trait]
impl HyperVTool for GetVmHostTool {
    const NAME: &'static str = "hyperv_get_vm_host";
    const DESCRIPTION: &'static str = "Gets a Hyper-V host.";
    type Input = GetVmHostInput;
    type Output = GetVmHostOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHost".to_string()];
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
             ComputerName, \
             LogicalProcessorCount, \
             @{{N='MemoryCapacity';E={{$_.MemoryCapacity.ToString()}}}}, \
             VirtualMachinePath, \
             VirtualHardDiskPath, \
             MaximumStorageMigrations, \
             MaximumVirtualMachineMigrations | ConvertTo-Json -Compress -Depth 3",
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
            output.push(VmHostInfo {
                computer_name: host["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                logical_processor_count: host["LogicalProcessorCount"].as_u64().unwrap_or_default()
                    as u32,
                memory_capacity: host["MemoryCapacity"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                virtual_machine_path: host["VirtualMachinePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                virtual_hard_disk_path: host["VirtualHardDiskPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                maximum_storage_migrations: host["MaximumStorageMigrations"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                maximum_virtual_machine_migrations: host["MaximumVirtualMachineMigrations"]
                    .as_u64()
                    .unwrap_or_default() as u32,
            });
        }

        Ok(GetVmHostOutput { hosts: output })
    }
}

register_tool!(GetVmHostTool);
