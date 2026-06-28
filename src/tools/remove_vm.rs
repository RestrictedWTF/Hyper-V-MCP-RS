use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmInput {
    /// Name of the virtual machine to remove.
    pub name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Suppress confirmation prompts.
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedVmInfo {
    pub name: String,
    pub id: String,
    pub state: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmOutput {
    /// Virtual machines that were removed.
    pub removed: Vec<RemovedVmInfo>,
}

#[derive(Default)]
pub struct RemoveVmTool;

#[async_trait]
impl HyperVTool for RemoveVmTool {
    const NAME: &'static str = "hyperv_remove_vm";
    const DESCRIPTION: &'static str = "Deletes a virtual machine.";
    type Input = RemoveVmInput;
    type Output = RemoveVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VM".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if input.force {
            args.push("-Force".to_string());
        }

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

        let removed = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(removed.len());
        for vm in removed {
            let name = vm["Name"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing Name in sidecar output".to_string())
            })?;
            let id = vm["Id"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing Id in sidecar output".to_string())
            })?;
            let state = vm["State"].as_str().ok_or_else(|| {
                ToolError::InvalidInput("missing State in sidecar output".to_string())
            })?;
            output.push(RemovedVmInfo {
                name: name.to_string(),
                id: id.to_string(),
                state: state.to_string(),
            });
        }

        Ok(RemoveVmOutput { removed: output })
    }
}

register_tool!(RemoveVmTool);
