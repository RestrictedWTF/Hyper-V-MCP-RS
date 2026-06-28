use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct ImportVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportVmInput {
    /// Path to the virtual machine to import.
    #[serde(rename = "path")]
    pub path: String,
    /// Generate a new ID for the imported virtual machine.
    #[serde(default, rename = "generateNewId")]
    pub generate_new_id: Option<bool>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ImportVmOutput {
    pub vms: Vec<ImportVmInfo>,
}


#[derive(Default)]
pub struct ImportVmTool;

#[async_trait]
impl HyperVTool for ImportVmTool {
    const NAME: &'static str = "hyperv_import_vm";
    const DESCRIPTION: &'static str = "Imports a virtual machine from a file.";
    type Input = ImportVmInput;
    type Output = ImportVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Import-VM".to_string()];
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput("path must not be empty".to_string()));
        }
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
        if let Some(generate_new_id) = &input.generate_new_id {
            if *generate_new_id {
                args.push("-GenerateNewId".to_string());
            }
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

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
            output.push(ImportVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }


        Ok(ImportVmOutput { vms: output })
    }
}

register_tool!(ImportVmTool);
