use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameVmSnapshotInput {
    /// Name of the virtual machine whose checkpoint is to be renamed.
    pub vm_name: String,
    /// Name of the checkpoint to be renamed.
    pub snapshot_name: String,
    /// New name for the checkpoint.
    pub new_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SnapshotInfo {
    pub name: String,
    pub id: String,
    pub vm_name: String,
    pub creation_time: String,
    pub snapshot_type: String,
    pub parent_snapshot_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RenameVmSnapshotOutput {
    pub snapshots: Vec<SnapshotInfo>,
}

#[derive(Default)]
pub struct RenameVmSnapshotTool;

#[async_trait]
impl HyperVTool for RenameVmSnapshotTool {
    const NAME: &'static str = "hyperv_rename_vm_snapshot";
    const DESCRIPTION: &'static str = "Renames a virtual machine checkpoint.";
    type Input = RenameVmSnapshotInput;
    type Output = RenameVmSnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.snapshot_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Snapshot name must not be empty".to_string(),
            ));
        }
        if input.new_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "New name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Rename-VMSnapshot".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        args.push(format!(
            "-Name '{}'",
            escape_ps_string(&input.snapshot_name)
        ));
        args.push(format!("-NewName '{}'", escape_ps_string(&input.new_name)));
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name, Id, VMName, \
             @{{N='CreationTime';E={{$_.CreationTime.ToString()}}}}, \
             @{{N='SnapshotType';E={{$_.SnapshotType.ToString()}}}}, \
             ParentSnapshotName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let snapshots = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(snapshots.len());
        for snap in snapshots {
            output.push(SnapshotInfo {
                name: snap["Name"].as_str().unwrap_or_default().to_string(),
                id: snap["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: snap["VMName"].as_str().unwrap_or_default().to_string(),
                creation_time: snap["CreationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                snapshot_type: snap["SnapshotType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                parent_snapshot_name: snap["ParentSnapshotName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(RenameVmSnapshotOutput { snapshots: output })
    }
}

register_tool!(RenameVmSnapshotTool);
