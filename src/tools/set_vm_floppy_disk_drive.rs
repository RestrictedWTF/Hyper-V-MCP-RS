use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmFloppyDiskDriveInput {
    /// Name of the virtual machine whose floppy disk drive is to be configured.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Path to the virtual floppy disk file to attach. Pass an empty string to dismount the drive.
    #[serde(default)]
    pub path: Option<String>,
    /// Name of the resource pool to which the virtual floppy disk drive is to be associated.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FloppyDiskDriveInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub path: String,
    #[serde(rename = "poolName")]
    pub pool_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmFloppyDiskDriveOutput {
    pub drives: Vec<FloppyDiskDriveInfo>,
}

#[derive(Default)]
pub struct SetVmFloppyDiskDriveTool;

#[async_trait]
impl HyperVTool for SetVmFloppyDiskDriveTool {
    const NAME: &'static str = "hyperv_set_vm_floppy_disk_drive";
    const DESCRIPTION: &'static str = "Configures a virtual floppy disk drive.";
    type Input = SetVmFloppyDiskDriveInput;
    type Output = SetVmFloppyDiskDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMFloppyDiskDrive -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(path) = &input.path {
            if path.trim().is_empty() {
                args.push("-Path $null".to_string());
            } else {
                args.push(format!("-Path '{}'", escape_ps_string(path)));
            }
        }
        if let Some(pool) = &input.resource_pool_name {
            args.push(format!("-ResourcePoolName '{}'", escape_ps_string(pool)));
        }

        let ps = format!(
            "{} | Select-Object Name, \
             @{{N='Id';E={{$_.Id.ToString()}}}}, \
             VMName, ComputerName, Path, PoolName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
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
            output.push(FloppyDiskDriveInfo {
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                id: drive["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: drive["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: drive["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                pool_name: drive["PoolName"].as_str().unwrap_or_default().to_string(),
                is_deleted: drive["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(SetVmFloppyDiskDriveOutput { drives: output })
    }
}

register_tool!(SetVmFloppyDiskDriveTool);
