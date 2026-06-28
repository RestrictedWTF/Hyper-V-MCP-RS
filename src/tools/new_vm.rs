use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewVmInput {
    /// Name of the new virtual machine.
pub name: String,
    /// Amount of memory, in bytes, to assign to the virtual machine.
    pub memory_startup_bytes: u64,
    /// Generation of the virtual machine. Defaults to 1 if omitted.
    #[serde(default)]
    pub generation: Option<u32>,
    /// Path to a new virtual hard disk file to create for the VM.
    #[serde(default, rename = "newVHDPath")]
    pub new_vhd_path: Option<String>,
    /// Size, in bytes, of the new virtual hard disk.
    #[serde(default, rename = "newVHDSizeBytes")]
    pub new_vhd_size_bytes: Option<u64>,
    /// Path where the virtual machine configuration files should be stored.
    #[serde(default)]
    pub path: Option<String>,
    /// Name of the virtual switch to connect the VM's network adapter to.
    #[serde(default, rename = "switchName")]
    pub switch_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct NewVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_startup_bytes: String,
    pub generation: u32,
    pub path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct NewVmOutput {
    pub vms: Vec<NewVmInfo>,
}

#[derive(Default)]
pub struct NewVmTool;

#[async_trait]
impl HyperVTool for NewVmTool {
    const NAME: &'static str = "hyperv_new_vm";
    const DESCRIPTION: &'static str = "Creates a new virtual machine.";
    type Input = NewVmInput;
    type Output = NewVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.memory_startup_bytes == 0 {
            return Err(ToolError::InvalidInput(
                "MemoryStartupBytes must be greater than 0".to_string(),
            ));
        }

        let mut args = vec!["New-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        args.push(format!(
            "-MemoryStartupBytes {}",
            input.memory_startup_bytes
        ));

        if let Some(generation) = input.generation {
            args.push(format!("-Generation {}", generation));
        }
        if let Some(vhd_path) = &input.new_vhd_path {
            if vhd_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "NewVHDPath must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-NewVHDPath '{}'", escape_ps_string(vhd_path)));
        }
        if let Some(vhd_size) = input.new_vhd_size_bytes {
            args.push(format!("-NewVHDSizeBytes {}", vhd_size));
        }
        if let Some(path) = &input.path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }
        if let Some(switch) = &input.switch_name {
            if switch.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SwitchName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-SwitchName '{}'", escape_ps_string(switch)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name, Id, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Uptime';E={{$_.Uptime.ToString()}}}}, \
             ProcessorCount, \
             @{{N='MemoryStartupBytes';E={{$_.MemoryStartupBytes.ToString()}}}}, \
             Generation, Path | ConvertTo-Json -Compress -Depth 3",
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
            output.push(NewVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_startup_bytes: vm["MemoryStartupBytes"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                generation: vm["Generation"].as_u64().unwrap_or_default() as u32,
                path: vm["Path"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(NewVmOutput { vms: output })
    }
}

register_tool!(NewVmTool);
