use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveVmStorageInput {
    /// Name of the virtual machine whose storage is to be moved.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Destination path for all virtual machine storage.
    #[serde(default, rename = "destinationStoragePath")]
    pub destination_storage_path: Option<String>,
    /// Destination path for the Smart Paging file.
    #[serde(default, rename = "smartPagingFilePath")]
    pub smart_paging_file_path: Option<String>,
    /// Destination path for snapshot files.
    #[serde(default, rename = "snapshotFilePath")]
    pub snapshot_file_path: Option<String>,
    /// Destination path for virtual hard disk files.
    #[serde(default, rename = "virtualHardDiskPath")]
    pub virtual_hard_disk_path: Option<String>,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
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
pub struct MoveVmStorageOutput {
    pub vms: Vec<MovedVmInfo>,
}

#[derive(Default)]
pub struct MoveVmStorageTool;

#[async_trait]
impl HyperVTool for MoveVmStorageTool {
    const NAME: &'static str = "hyperv_move_vm_storage";
    const DESCRIPTION: &'static str = "Moves the storage of a virtual machine.";
    type Input = MoveVmStorageInput;
    type Output = MoveVmStorageOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VMName must not be empty".to_string(),
            ));
        }

        if input.destination_storage_path.is_none()
            && input.smart_paging_file_path.is_none()
            && input.snapshot_file_path.is_none()
            && input.virtual_hard_disk_path.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one destination path must be provided".to_string(),
            ));
        }

        if let Some(path) = &input.destination_storage_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "DestinationStoragePath must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(path) = &input.smart_paging_file_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SmartPagingFilePath must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(path) = &input.snapshot_file_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SnapshotFilePath must not be empty when provided".to_string(),
                ));
            }
        }
        if let Some(path) = &input.virtual_hard_disk_path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VirtualHardDiskPath must not be empty when provided".to_string(),
                ));
            }
        }

        let mut args = vec![format!(
            "Move-VMStorage -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(path) = &input.destination_storage_path {
            args.push(format!(
                "-DestinationStoragePath '{}'",
                escape_ps_string(path)
            ));
        }
        if let Some(path) = &input.smart_paging_file_path {
            args.push(format!("-SmartPagingFilePath '{}'", escape_ps_string(path)));
        }
        if let Some(path) = &input.snapshot_file_path {
            args.push(format!("-SnapshotFilePath '{}'", escape_ps_string(path)));
        }
        if let Some(path) = &input.virtual_hard_disk_path {
            args.push(format!("-VirtualHardDiskPath '{}'", escape_ps_string(path)));
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

        Ok(MoveVmStorageOutput { vms: output })
    }
}

register_tool!(MoveVmStorageTool);
