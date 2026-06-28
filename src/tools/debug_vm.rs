use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DebugVmInput {
    /// Name of the virtual machine to debug.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Forces the command to run without asking for user confirmation.
    #[serde(default)]
    pub force: bool,
    /// Sends a nonmaskable interrupt (NMI) to the virtual machine.
    #[serde(default, rename = "injectNonMaskableInterrupt")]
    pub inject_non_maskable_interrupt: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DebugVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
    pub uptime: String,
    pub processor_count: u32,
    pub memory_assigned: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DebugVmOutput {
    pub vms: Vec<DebugVmInfo>,
}

#[derive(Default)]
pub struct DebugVmTool;

#[async_trait]
impl HyperVTool for DebugVmTool {
    const NAME: &'static str = "hyperv_debug_vm";
    const DESCRIPTION: &'static str = "Debugs a virtual machine.";
    type Input = DebugVmInput;
    type Output = DebugVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Debug-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if input.force {
            args.push("-Force".to_string());
        }
        if input.inject_non_maskable_interrupt {
            args.push("-InjectNonMaskableInterrupt".to_string());
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
            output.push(DebugVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
                uptime: vm["Uptime"].as_str().unwrap_or_default().to_string(),
                processor_count: vm["ProcessorCount"].as_u64().unwrap_or_default() as u32,
                memory_assigned: vm["MemoryAssigned"].as_u64().unwrap_or_default(),
            });
        }

        Ok(DebugVmOutput { vms: output })
    }
}

register_tool!(DebugVmTool);
