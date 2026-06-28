use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHardDiskDriveInput {
    /// Name of the virtual machine whose hard disk drives are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the hard disk drive to retrieve. If omitted, all hard disk drives are returned.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Unique identifier of the hard disk drive to retrieve.
    #[serde(default)]
    pub id: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct HardDiskDriveInfo {
    pub name: String,
    pub id: String,
    pub vm_name: String,
    pub vm_id: String,
    pub controller_type: String,
    pub controller_number: u32,
    pub controller_location: u32,
    pub path: String,
    pub pool_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHardDiskDriveOutput {
    pub hard_disk_drives: Vec<HardDiskDriveInfo>,
}

#[derive(Default)]
pub struct GetVmHardDiskDriveTool;

#[async_trait]
impl HyperVTool for GetVmHardDiskDriveTool {
    const NAME: &'static str = "hyperv_get_vm_hard_disk_drive";
    const DESCRIPTION: &'static str =
        "Gets the virtual hard disk drives attached to one or more virtual machines.";
    type Input = GetVmHardDiskDriveInput;
    type Output = GetVmHardDiskDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHardDiskDrive".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Hard disk drive name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(id) = &input.id {
            if id.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Hard disk drive id must not be empty".to_string(),
                ));
            }
            args.push(format!("-Id '{}'", escape_ps_string(id)));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, Id, VMName, VMId, \
             @{{N='ControllerType';E={{$_.ControllerType.ToString()}}}}, \
             ControllerNumber, ControllerLocation, Path, PoolName | \
             ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let drives = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(drives.len());
        for drive in drives {
            output.push(HardDiskDriveInfo {
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                id: drive["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: drive["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: drive["VMId"].as_str().unwrap_or_default().to_string(),
                controller_type: drive["ControllerType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                controller_number: drive["ControllerNumber"].as_u64().unwrap_or_default() as u32,
                controller_location: drive["ControllerLocation"].as_u64().unwrap_or_default()
                    as u32,
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                pool_name: drive["PoolName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(GetVmHardDiskDriveOutput {
            hard_disk_drives: output,
        })
    }
}

register_tool!(GetVmHardDiskDriveTool);
