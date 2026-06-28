use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmInput {
    /// Name of the virtual machine to configure.
pub name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// New name for the virtual machine.
    #[serde(default, rename = "newVMName")]
    pub new_name: Option<String>,
    /// Amount of memory, in bytes, allocated to the virtual machine at startup.
    #[serde(default, rename = "memoryStartupBytes")]
    pub memory_startup_bytes: Option<u64>,
    /// Number of virtual processors for the virtual machine.
    #[serde(default, rename = "processorCount")]
    pub processor_count: Option<u32>,
    /// Action the virtual machine takes on host start: Nothing, StartIfRunning, or Start.
    #[serde(default, rename = "automaticStartAction")]
    pub automatic_start_action: Option<String>,
    /// Action the virtual machine takes on host shutdown: TurnOff, Save, or ShutDown.
    #[serde(default, rename = "automaticStopAction")]
    pub automatic_stop_action: Option<String>,
    /// Number of seconds by which the virtual machine's start is delayed.
    #[serde(default, rename = "automaticStartDelay")]
    pub automatic_start_delay: Option<i32>,
    /// Action on critical error: None or Pause.
    #[serde(default, rename = "automaticCriticalErrorAction")]
    pub automatic_critical_error_action: Option<String>,
    /// Minutes to wait in critical pause before powering off.
    #[serde(default, rename = "automaticCriticalErrorActionTimeout")]
    pub automatic_critical_error_action_timeout: Option<i32>,
    /// Enables or disables automatic checkpoints.
    #[serde(default, rename = "automaticCheckpointsEnabled")]
    pub automatic_checkpoints_enabled: Option<bool>,
    /// Notes to associate with the virtual machine.
    #[serde(default)]
    pub notes: Option<String>,
    /// Folder in which snapshot files are stored.
    #[serde(default, rename = "snapshotFileLocation")]
    pub snapshot_file_location: Option<String>,
    /// Folder in which the Smart Paging file is stored.
    #[serde(default, rename = "smartPagingFilePath")]
    pub smart_paging_file_path: Option<String>,
    /// Checkpoint type: Disabled, Production, ProductionOnly, or Standard.
    #[serde(default, rename = "checkpointType")]
    pub checkpoint_type: Option<String>,
    /// Configure dynamic memory.
    #[serde(default, rename = "dynamicMemory")]
    pub dynamic_memory: Option<bool>,
    /// Configure static memory.
    #[serde(default, rename = "staticMemory")]
    pub static_memory: Option<bool>,
    /// Minimum memory, in bytes, for dynamic memory.
    #[serde(default, rename = "memoryMinimumBytes")]
    pub memory_minimum_bytes: Option<u64>,
    /// Maximum memory, in bytes, for dynamic memory.
    #[serde(default, rename = "memoryMaximumBytes")]
    pub memory_maximum_bytes: Option<u64>,
    /// Specifies whether guest controlled cache types are used.
    #[serde(default, rename = "guestControlledCacheTypes")]
    pub guest_controlled_cache_types: Option<bool>,
    /// Lock console on disconnect: On or Off.
    #[serde(default, rename = "lockOnDisconnect")]
    pub lock_on_disconnect: Option<String>,
    /// Low memory mapped I/O space.
    #[serde(default, rename = "lowMemoryMappedIoSpace")]
    pub low_memory_mapped_io_space: Option<u32>,
    /// High memory mapped I/O space.
    #[serde(default, rename = "highMemoryMappedIoSpace")]
    pub high_memory_mapped_io_space: Option<u64>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmConfig {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmOutput {
    pub vms: Vec<VmConfig>,
}

#[derive(Default)]
pub struct SetVmTool;

#[async_trait]
impl HyperVTool for SetVmTool {
    const NAME: &'static str = "hyperv_set_vm";
    const DESCRIPTION: &'static str = "Configures a virtual machine.";
    type Input = SetVmInput;
    type Output = SetVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!("Set-VM -Name '{}'", escape_ps_string(&input.name))];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(new_name) = &input.new_name {
            args.push(format!("-NewVMName '{}'", escape_ps_string(new_name)));
        }
        if let Some(bytes) = input.memory_startup_bytes {
            args.push(format!("-MemoryStartupBytes {}", bytes));
        }
        if let Some(count) = input.processor_count {
            args.push(format!("-ProcessorCount {}", count));
        }
        if let Some(action) = &input.automatic_start_action {
            args.push(format!(
                "-AutomaticStartAction '{}'",
                escape_ps_string(action)
            ));
        }
        if let Some(action) = &input.automatic_stop_action {
            args.push(format!(
                "-AutomaticStopAction '{}'",
                escape_ps_string(action)
            ));
        }
        if let Some(delay) = input.automatic_start_delay {
            args.push(format!("-AutomaticStartDelay {}", delay));
        }
        if let Some(action) = &input.automatic_critical_error_action {
            args.push(format!(
                "-AutomaticCriticalErrorAction '{}'",
                escape_ps_string(action)
            ));
        }
        if let Some(timeout) = input.automatic_critical_error_action_timeout {
            args.push(format!("-AutomaticCriticalErrorActionTimeout {}", timeout));
        }
        if let Some(enabled) = input.automatic_checkpoints_enabled {
            args.push(format!("-AutomaticCheckpointsEnabled ${}", enabled));
        }
        if let Some(notes) = &input.notes {
            args.push(format!("-Notes '{}'", escape_ps_string(notes)));
        }
        if let Some(path) = &input.snapshot_file_location {
            args.push(format!(
                "-SnapshotFileLocation '{}'",
                escape_ps_string(path)
            ));
        }
        if let Some(path) = &input.smart_paging_file_path {
            args.push(format!("-SmartPagingFilePath '{}'", escape_ps_string(path)));
        }
        if let Some(t) = &input.checkpoint_type {
            args.push(format!("-CheckpointType '{}'", escape_ps_string(t)));
        }
        if let Some(enabled) = input.dynamic_memory {
            args.push(format!("-DynamicMemory:${}", enabled));
        }
        if let Some(enabled) = input.static_memory {
            args.push(format!("-StaticMemory:${}", enabled));
        }
        if let Some(bytes) = input.memory_minimum_bytes {
            args.push(format!("-MemoryMinimumBytes {}", bytes));
        }
        if let Some(bytes) = input.memory_maximum_bytes {
            args.push(format!("-MemoryMaximumBytes {}", bytes));
        }
        if let Some(enabled) = input.guest_controlled_cache_types {
            args.push(format!("-GuestControlledCacheTypes ${}", enabled));
        }
        if let Some(state) = &input.lock_on_disconnect {
            args.push(format!("-LockOnDisconnect '{}'", escape_ps_string(state)));
        }
        if let Some(space) = input.low_memory_mapped_io_space {
            args.push(format!("-LowMemoryMappedIoSpace {}", space));
        }
        if let Some(space) = input.high_memory_mapped_io_space {
            args.push(format!("-HighMemoryMappedIoSpace {}", space));
        }

        args.push("-PassThru".to_string());

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
            output.push(VmConfig {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(SetVmOutput { vms: output })
    }
}

register_tool!(SetVmTool);
