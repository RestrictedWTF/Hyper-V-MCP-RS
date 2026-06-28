use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveVmInput {
    /// Name of the virtual machine to move.
    pub name: String,
    /// Hyper-V host to which the virtual machine is to be moved.
    #[serde(rename = "destinationHost")]
    pub destination_host: String,
    /// Hyper-V host on which the virtual machine currently resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Specifies that both the virtual machine and its storage are to be moved.
    #[serde(default, rename = "includeStorage")]
    pub include_storage: bool,
    /// Destination path to which all virtual machine data is to be moved.
    #[serde(default, rename = "destinationStoragePath")]
    pub destination_storage_path: Option<String>,
    /// Path for the virtual machine configuration files on the destination host.
    #[serde(default, rename = "virtualMachinePath")]
    pub virtual_machine_path: Option<String>,
    /// Path for snapshot files on the destination host.
    #[serde(default, rename = "snapshotFilePath")]
    pub snapshot_file_path: Option<String>,
    /// Path for the smart paging file on the destination host.
    #[serde(default, rename = "smartPagingFilePath")]
    pub smart_paging_file_path: Option<String>,
    /// Name of the processor resource pool to be used.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
    /// Indicates that Hyper-V retains the parent virtual hard disk on the source.
    #[serde(default, rename = "retainVhdCopiesOnSource")]
    pub retain_vhd_copies_on_source: bool,
    /// Indicates that Hyper-V deletes the parent virtual hard disk on the source after moving a differencing disk.
    #[serde(default, rename = "removeSourceUnmanagedVhds")]
    pub remove_source_unmanaged_vhds: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MovedVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MoveVmOutput {
    pub vms: Vec<MovedVmInfo>,
}

#[derive(Default)]
pub struct MoveVmTool;

#[async_trait]
impl HyperVTool for MoveVmTool {
    const NAME: &'static str = "hyperv_move_vm";
    const DESCRIPTION: &'static str = "Moves a virtual machine to a new Hyper-V host.";
    type Input = MoveVmInput;
    type Output = MoveVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.destination_host.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "destinationHost must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Move-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        args.push(format!(
            "-DestinationHost '{}'",
            escape_ps_string(&input.destination_host)
        ));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if input.include_storage {
            args.push("-IncludeStorage".to_string());
        }
        if let Some(path) = &input.destination_storage_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "destinationStoragePath must not be empty".to_string(),
                ));
            }
            args.push(format!(
                "-DestinationStoragePath '{}'",
                escape_ps_string(path)
            ));
        }
        if let Some(path) = &input.virtual_machine_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "virtualMachinePath must not be empty".to_string(),
                ));
            }
            args.push(format!("-VirtualMachinePath '{}'", escape_ps_string(path)));
        }
        if let Some(path) = &input.snapshot_file_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "snapshotFilePath must not be empty".to_string(),
                ));
            }
            args.push(format!("-SnapshotFilePath '{}'", escape_ps_string(path)));
        }
        if let Some(path) = &input.smart_paging_file_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "smartPagingFilePath must not be empty".to_string(),
                ));
            }
            args.push(format!("-SmartPagingFilePath '{}'", escape_ps_string(path)));
        }
        if let Some(pool) = &input.resource_pool_name {
            if pool.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "resourcePoolName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ResourcePoolName '{}'", escape_ps_string(pool)));
        }
        if input.retain_vhd_copies_on_source {
            args.push("-RetainVhdCopiesOnSource".to_string());
        }
        if input.remove_source_unmanaged_vhds {
            args.push("-RemoveSourceUnmanagedVhds".to_string());
        }

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
            output.push(MovedVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(MoveVmOutput { vms: output })
    }
}

register_tool!(MoveVmTool);
