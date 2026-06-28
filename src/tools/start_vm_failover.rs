use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StartVmFailoverInput {
    /// Name of the virtual machine on which to start failover.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Prepares the primary virtual machine for a planned failover and replicates pending changes.
    #[serde(default)]
    pub prepare: bool,
    /// Performs a test failover using the chosen recovery point.
    #[serde(default, rename = "asTest")]
    pub as_test: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct FailoverVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StartVmFailoverOutput {
    pub vms: Vec<FailoverVmInfo>,
}

#[derive(Default)]
pub struct StartVmFailoverTool;

#[async_trait]
impl HyperVTool for StartVmFailoverTool {
    const NAME: &'static str = "hyperv_start_vm_failover";
    const DESCRIPTION: &'static str = "Starts failover on a virtual machine.";
    type Input = StartVmFailoverInput;
    type Output = StartVmFailoverOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Start-VMFailover".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if input.prepare {
            args.push("-Prepare".to_string());
        }

        if input.as_test {
            args.push("-AsTest".to_string());
        }

        args.push("-Passthru".to_string());
        args.push("-Confirm:$false".to_string());

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
            output.push(FailoverVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(StartVmFailoverOutput { vms: output })
    }
}

register_tool!(StartVmFailoverTool);
