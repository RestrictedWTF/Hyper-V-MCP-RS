use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportVmInitialReplicationInput {
    /// Name of the virtual machine for which the initial replication files are to be imported.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Path of the initial replication files to import.
    pub path: String,
    /// Hyper-V host on which to import the initial replication files. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ImportedVmInitialReplicationInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ImportVmInitialReplicationOutput {
    /// Virtual machines whose initial replication files were imported.
    pub vms: Vec<ImportedVmInitialReplicationInfo>,
}

#[derive(Default)]
pub struct ImportVmInitialReplicationTool;

#[async_trait]
impl HyperVTool for ImportVmInitialReplicationTool {
    const NAME: &'static str = "hyperv_import_vminitialreplication";
    const DESCRIPTION: &'static str =
        "Imports initial replication files for a Replica virtual machine to complete the initial replication when using external media as the source.";
    type Input = ImportVmInitialReplicationInput;
    type Output = ImportVmInitialReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Import-VMInitialReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Uptime';E={{$_.Uptime.ToString()}}}}, \
             ProcessorCount, MemoryAssigned | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let vms = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(vms.len());
        for vm in vms {
            output.push(ImportedVmInitialReplicationInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(ImportVmInitialReplicationOutput { vms: output })
    }
}

register_tool!(ImportVmInitialReplicationTool);
