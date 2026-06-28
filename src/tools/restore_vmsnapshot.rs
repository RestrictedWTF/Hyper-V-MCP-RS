use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RestoreVmsnapshotInput {
    /// Name of the checkpoint to restore.
    #[serde(rename = "snapshotName")]
    pub snapshot_name: String,
    /// Name of the virtual machine whose checkpoint is being restored.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RestoredVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RestoreVmsnapshotOutput {
    /// Virtual machines that were restored to the checkpoint.
    pub vms: Vec<RestoredVmInfo>,
}

#[derive(Default)]
pub struct RestoreVmsnapshotTool;

#[async_trait]
impl HyperVTool for RestoreVmsnapshotTool {
    const NAME: &'static str = "hyperv_restore_vmsnapshot";
    const DESCRIPTION: &'static str = "Restores a virtual machine checkpoint.";
    type Input = RestoreVmsnapshotInput;
    type Output = RestoreVmsnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.snapshot_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "snapshotName must not be empty".to_string(),
            ));
        }
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Restore-VMSnapshot".to_string()];
        args.push(format!(
            "-Name '{}'",
            escape_ps_string(&input.snapshot_name)
        ));
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Confirm:$false".to_string());
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
            let name = vm["Name"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing Name in sidecar output".to_string())
            })?;
            let id = vm["Id"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing Id in sidecar output".to_string())
            })?;
            let state = vm["State"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing State in sidecar output".to_string())
            })?;
            output.push(RestoredVmInfo {
                name: name.to_string(),
                id: id.to_string(),
                state: state.to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(RestoreVmsnapshotOutput { vms: output })
    }
}

register_tool!(RestoreVmsnapshotTool);
