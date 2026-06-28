use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmSnapshotInput {
    /// Name of the virtual machine whose checkpoint is being removed.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the checkpoint to remove.
    #[serde(rename = "snapshotName")]
    pub snapshot_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedSnapshotInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "snapshotType")]
    pub snapshot_type: String,
    #[serde(rename = "creationTime")]
    pub creation_time: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmSnapshotOutput {
    /// Checkpoints that were removed.
    pub removed: Vec<RemovedSnapshotInfo>,
}

#[derive(Default)]
pub struct RemoveVmSnapshotTool;

#[async_trait]
impl HyperVTool for RemoveVmSnapshotTool {
    const NAME: &'static str = "hyperv_remove_vm_snapshot";
    const DESCRIPTION: &'static str = "Deletes a virtual machine checkpoint.";
    type Input = RemoveVmSnapshotInput;
    type Output = RemoveVmSnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }
        if input.snapshot_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "snapshotName must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMSnapshot".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        args.push(format!(
            "-Name '{}'",
            escape_ps_string(&input.snapshot_name)
        ));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

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

        let snapshots = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut removed = Vec::with_capacity(snapshots.len());
        for snapshot in snapshots {
            removed.push(RemovedSnapshotInfo {
                name: snapshot["Name"].as_str().unwrap_or_default().to_string(),
                id: snapshot["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: snapshot["VMName"].as_str().unwrap_or_default().to_string(),
                snapshot_type: snapshot["SnapshotType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                creation_time: snapshot["CreationTime"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(RemoveVmSnapshotOutput { removed })
    }
}

register_tool!(RemoveVmSnapshotTool);
