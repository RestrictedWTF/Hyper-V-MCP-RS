use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckpointVmInput {
    /// Name of the virtual machine to checkpoint.
    pub name: Option<String>,
    /// Name of the checkpoint. If omitted, Hyper-V generates a name.
    #[serde(default, rename = "snapshotName")]
    pub snapshot_name: Option<String>,
    /// Type of checkpoint to create. Standard or Production.
    #[serde(default, rename = "snapshotType")]
    pub snapshot_type: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CheckpointInfo {
    pub name: String,
    pub id: String,
    pub vm_name: String,
    pub snapshot_type: String,
    pub creation_time: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CheckpointVmOutput {
    pub checkpoints: Vec<CheckpointInfo>,
}

#[derive(Default)]
pub struct CheckpointVmTool;

#[async_trait]
impl HyperVTool for CheckpointVmTool {
    const NAME: &'static str = "hyperv_checkpoint_vm";
    const DESCRIPTION: &'static str = "Creates a checkpoint of a virtual machine.";
    type Input = CheckpointVmInput;
    type Output = CheckpointVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let name = input
            .name
            .ok_or_else(|| ToolError::InvalidInput("name is required".to_string()))?;
        if name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Checkpoint-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&name)));

        if let Some(snapshot_name) = &input.snapshot_name {
            if snapshot_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SnapshotName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-SnapshotName '{}'",
                escape_ps_string(snapshot_name)
            ));
        }
        if let Some(snapshot_type) = &input.snapshot_type {
            if snapshot_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "SnapshotType must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-SnapshotType '{}'",
                escape_ps_string(snapshot_type)
            ));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, VMName, \
             @{{N='SnapshotType';E={{$_.SnapshotType.ToString()}}}}, \
             @{{N='CreationTime';E={{$_.CreationTime.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let checkpoints = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(checkpoints.len());
        for checkpoint in checkpoints {
            output.push(CheckpointInfo {
                name: checkpoint["Name"].as_str().unwrap_or_default().to_string(),
                id: checkpoint["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: checkpoint["VMName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                snapshot_type: checkpoint["SnapshotType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                creation_time: checkpoint["CreationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(CheckpointVmOutput {
            checkpoints: output,
        })
    }
}

register_tool!(CheckpointVmTool);
