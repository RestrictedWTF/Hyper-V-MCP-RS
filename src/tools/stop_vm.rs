use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

/// How to stop the virtual machine.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StopVmAction {
    /// Shut down the VM gracefully through the guest OS (default).
    #[default]
    Shutdown,
    /// Save the VM state.
    Save,
    /// Turn the VM off immediately (like pulling the power cord).
    TurnOff,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StopVmInput {
    /// Name of the virtual machine to stop.
pub name: String,
    /// Stop action: "shutdown" (default), "save", or "turnoff".
    #[serde(default)]
    pub action: StopVmAction,
    /// Force the operation without prompting. Effective for shutdown and save.
    #[serde(default)]
    pub force: bool,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StoppedVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StopVmOutput {
    pub stopped_vms: Vec<StoppedVmInfo>,
}

#[derive(Default)]
pub struct StopVmTool;

#[async_trait]
impl HyperVTool for StopVmTool {
    const NAME: &'static str = "hyperv_stop_vm";
    const DESCRIPTION: &'static str = "Shuts down, turns off, or saves a virtual machine.";
    type Input = StopVmInput;
    type Output = StopVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Stop-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));

        match input.action {
            StopVmAction::Save => args.push("-Save".to_string()),
            StopVmAction::TurnOff => args.push("-TurnOff".to_string()),
            StopVmAction::Shutdown => {
                // Default Stop-VM behavior is a graceful guest shutdown.
            }
        }

        if input.force {
            args.push("-Force".to_string());
        }

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
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

        let mut stopped_vms = Vec::with_capacity(vms.len());
        for vm in vms {
            stopped_vms.push(StoppedVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(StopVmOutput { stopped_vms })
    }
}

register_tool!(StopVmTool);
