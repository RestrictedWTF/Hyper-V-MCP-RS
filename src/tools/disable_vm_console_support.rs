use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisableVmConsoleSupportInput {
    /// Name of the generation 2 virtual machine whose console support is to be disabled.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisabledVmConsoleSupportInfo {
    pub name: String,
    pub id: String,
    pub state: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisableVmConsoleSupportOutput {
    /// Virtual machines whose console support was disabled.
    #[serde(rename = "disabledVms")]
    pub disabled_vms: Vec<DisabledVmConsoleSupportInfo>,
}

#[derive(Default)]
pub struct DisableVmConsoleSupportTool;

#[async_trait]
impl HyperVTool for DisableVmConsoleSupportTool {
    const NAME: &'static str = "hyperv_disable_vm_console_support";
    const DESCRIPTION: &'static str =
        "Disables keyboard, video, and mouse for a generation 2 virtual machine.";
    type Input = DisableVmConsoleSupportInput;
    type Output = DisableVmConsoleSupportOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Disable-VMConsoleSupport".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.name)));

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

        let mut disabled_vms = Vec::with_capacity(vms.len());
        for vm in vms {
            disabled_vms.push(DisabledVmConsoleSupportInfo {
                name: vm["Name"].as_str().unwrap_or_default().to_string(),
                id: vm["Id"].as_str().unwrap_or_default().to_string(),
                state: vm["State"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(DisableVmConsoleSupportOutput { disabled_vms })
    }
}

register_tool!(DisableVmConsoleSupportTool);
