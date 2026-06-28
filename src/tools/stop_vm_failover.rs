use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopVmFailoverInput {
    /// Name of the virtual machine whose failover should be stopped.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StoppedVmFailoverInfo {
    pub name: String,
    pub id: String,
    pub state: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StopVmFailoverOutput {
    pub vms: Vec<StoppedVmFailoverInfo>,
}

#[derive(Default)]
pub struct StopVmFailoverTool;

#[async_trait]
impl HyperVTool for StopVmFailoverTool {
    const NAME: &'static str = "hyperv_stop_vm_failover";
    const DESCRIPTION: &'static str = "Stops failover of a virtual machine.";
    type Input = StopVmFailoverInput;
    type Output = StopVmFailoverOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Stop-VMFailover".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.name)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, \
             @{{N='State';E={{$_.State.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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
            output.push(StoppedVmFailoverInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(StopVmFailoverOutput { vms: output })
    }
}

register_tool!(StopVmFailoverTool);
