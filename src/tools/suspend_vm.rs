use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SuspendVmInput {
    /// Name of the virtual machine to suspend.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SuspendedVmInfo {
    pub name: String,
    pub state: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SuspendVmOutput {
    pub suspended_vms: Vec<SuspendedVmInfo>,
}

#[derive(Default)]
pub struct SuspendVmTool;

#[async_trait]
impl HyperVTool for SuspendVmTool {
    const NAME: &'static str = "hyperv_suspend_vm";
    const DESCRIPTION: &'static str = "Suspends, or pauses, a virtual machine.";
    type Input = SuspendVmInput;
    type Output = SuspendVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(ToolError::InvalidInput(
                "name is required to suspend a virtual machine".to_string(),
            ));
        }

        let mut args = vec!["Suspend-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(name)));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name, \
             @{{N='State';E={{$_.State.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );
        // Note: State is a .NET enum. It is forced to a string via a calculated
        // Select-Object property so serde_json sees a string value.

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
            output.push(SuspendedVmInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SuspendVmOutput {
            suspended_vms: output,
        })
    }
}

register_tool!(SuspendVmTool);
