use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct RenameVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameVmInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// New name for the virtual machine.
    #[serde(rename = "newName")]
    pub new_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RenameVmOutput {
    pub vms: Vec<RenameVmInfo>,
}


#[derive(Default)]
pub struct RenameVmTool;

#[async_trait]
impl HyperVTool for RenameVmTool {
    const NAME: &'static str = "hyperv_rename_vm";
    const DESCRIPTION: &'static str = "Renames a virtual machine.";
    type Input = RenameVmInput;
    type Output = RenameVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Rename-VM".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if input.new_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("new_name must not be empty".to_string()));
        }
        args.push(format!("-NewName '{}'", escape_ps_string(&input.new_name)));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        args.push("-PassThru".to_string());
        let ps = format!("{} | Select-Object Name, Id, @{{N='State';E={{$_.State.ToString()}}}}, @{{N='Uptime';E={{$_.Uptime.ToString()}}}}, ProcessorCount, MemoryAssigned | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(RenameVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }


        Ok(RenameVmOutput { vms: output })
    }
}

register_tool!(RenameVmTool);
