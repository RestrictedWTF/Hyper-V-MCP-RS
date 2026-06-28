use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartVmInitialReplicationInput {
    /// Name of the virtual machine for which to start initial replication.
    /// Supports a single VM name or wildcard such as *.
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Path to store the initial replication files when using external media.
    #[serde(default, rename = "destinationPath")]
    pub destination_path: Option<String>,
    /// Scheduled start time for the initial replication, up to 7 days in the future.
    #[serde(default, rename = "initialReplicationStartTime")]
    pub initial_replication_start_time: Option<String>,
    /// Use a copy of the virtual machine on the Replica server as the basis
    /// for the initial replication.
    #[serde(default, rename = "useBackup")]
    pub use_backup: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmInitialReplicationInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StartVmInitialReplicationOutput {
    pub vms: Vec<VmInitialReplicationInfo>,
}

#[derive(Default)]
pub struct StartVmInitialReplicationTool;

#[async_trait]
impl HyperVTool for StartVmInitialReplicationTool {
    const NAME: &'static str = "hyperv_start_vm_initial_replication";
    const DESCRIPTION: &'static str = "Starts replication of a virtual machine.";
    type Input = StartVmInitialReplicationInput;
    type Output = StartVmInitialReplicationOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let vm_name = input
            .vm_name
            .ok_or_else(|| ToolError::InvalidInput("vm_name is required".to_string()))?;

        if vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Start-VMInitialReplication".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&vm_name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(path) = &input.destination_path {
            args.push(format!("-DestinationPath '{}'", escape_ps_string(path)));
        }

        if let Some(time) = &input.initial_replication_start_time {
            args.push(format!(
                "-InitialReplicationStartTime '{}'",
                escape_ps_string(time)
            ));
        }

        if input.use_backup {
            args.push("-UseBackup".to_string());
        }

        args.push("-Passthru".to_string());

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
            output.push(VmInitialReplicationInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(StartVmInitialReplicationOutput { vms: output })
    }
}

register_tool!(StartVmInitialReplicationTool);
