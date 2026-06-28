use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmMemoryInput {
    /// Name of the virtual machine whose memory is to be configured.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Amount of memory, in bytes, allocated to the virtual machine at startup.
    #[serde(default, rename = "startupBytes")]
    pub startup_bytes: Option<u64>,
    /// Minimum amount of memory, in bytes, that can be allocated to the virtual machine when dynamic memory is enabled.
    #[serde(default, rename = "minimumBytes")]
    pub minimum_bytes: Option<u64>,
    /// Maximum amount of memory, in bytes, that can be allocated to the virtual machine when dynamic memory is enabled.
    #[serde(default, rename = "maximumBytes")]
    pub maximum_bytes: Option<u64>,
    /// Enables or disables dynamic memory for the virtual machine.
    #[serde(default, rename = "dynamicMemoryEnabled")]
    pub dynamic_memory_enabled: Option<bool>,
    /// Percentage of extra memory to reserve for the virtual machine, from 5 to 2000.
    #[serde(default)]
    pub buffer: Option<u32>,
    /// Priority for memory allocation among virtual machines, from 1 to 10000.
    #[serde(default)]
    pub priority: Option<u32>,
    /// Folder in which the Smart Paging file is stored.
    #[serde(default, rename = "smartPagingFilePath")]
    pub smart_paging_file_path: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmMemoryInfo {
    pub vm_name: String,
    pub computer_name: String,
    /// Startup memory, in bytes.
    pub startup: String,
    /// Minimum memory, in bytes.
    pub minimum: String,
    /// Maximum memory, in bytes.
    pub maximum: String,
    pub dynamic_memory_enabled: bool,
    /// Memory buffer percentage.
    pub buffer: u32,
    /// Memory priority.
    pub priority: u32,
    pub smart_paging_file_path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmMemoryOutput {
    pub memory: Vec<VmMemoryInfo>,
}

#[derive(Default)]
pub struct SetVmMemoryTool;

#[async_trait]
impl HyperVTool for SetVmMemoryTool {
    const NAME: &'static str = "hyperv_set_vm_memory";
    const DESCRIPTION: &'static str = "Configures the memory of a virtual machine.";
    type Input = SetVmMemoryInput;
    type Output = SetVmMemoryOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMMemory -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(bytes) = input.startup_bytes {
            args.push(format!("-StartupBytes {}", bytes));
        }
        if let Some(bytes) = input.minimum_bytes {
            args.push(format!("-MinimumBytes {}", bytes));
        }
        if let Some(bytes) = input.maximum_bytes {
            args.push(format!("-MaximumBytes {}", bytes));
        }
        if let Some(enabled) = input.dynamic_memory_enabled {
            args.push(format!("-DynamicMemoryEnabled ${}", enabled));
        }
        if let Some(buffer) = input.buffer {
            args.push(format!("-Buffer {}", buffer));
        }
        if let Some(priority) = input.priority {
            args.push(format!("-Priority {}", priority));
        }
        if let Some(path) = &input.smart_paging_file_path {
            args.push(format!("-SmartPagingFilePath '{}'", escape_ps_string(path)));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             VMName, ComputerName, \
             @{{N='Startup';E={{$_.Startup.ToString()}}}}, \
             @{{N='Minimum';E={{$_.Minimum.ToString()}}}}, \
             @{{N='Maximum';E={{$_.Maximum.ToString()}}}}, \
             DynamicMemoryEnabled, Buffer, Priority, \
             @{{N='SmartPagingFilePath';E={{$_.SmartPagingFilePath.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let memory = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(memory.len());
        for m in memory {
            output.push(VmMemoryInfo {
                vm_name: m["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: m["ComputerName"].as_str().unwrap_or_default().to_string(),
                startup: m["Startup"].as_str().unwrap_or_default().to_string(),
                minimum: m["Minimum"].as_str().unwrap_or_default().to_string(),
                maximum: m["Maximum"].as_str().unwrap_or_default().to_string(),
                dynamic_memory_enabled: m["DynamicMemoryEnabled"].as_bool().unwrap_or_default(),
                buffer: m["Buffer"].as_u64().unwrap_or_default() as u32,
                priority: m["Priority"].as_u64().unwrap_or_default() as u32,
                smart_paging_file_path: m["SmartPagingFilePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmMemoryOutput { memory: output })
    }
}

register_tool!(SetVmMemoryTool);
