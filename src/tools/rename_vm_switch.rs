use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameVmSwitchInput {
    /// Name of the virtual switch to rename.
    pub name: String,
    /// New name for the virtual switch.
    #[serde(rename = "newName")]
    pub new_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VMSwitchInfo {
    pub name: String,
    pub id: String,
    pub switch_type: String,
    pub status: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RenameVmSwitchOutput {
    pub switches: Vec<VMSwitchInfo>,
}

#[derive(Default)]
pub struct RenameVmSwitchTool;

#[async_trait]
impl HyperVTool for RenameVmSwitchTool {
    const NAME: &'static str = "hyperv_rename_vm_switch";
    const DESCRIPTION: &'static str = "Renames a virtual switch.";
    type Input = RenameVmSwitchInput;
    type Output = RenameVmSwitchOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Switch name must not be empty".to_string(),
            ));
        }
        if input.new_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "New name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Rename-VMSwitch".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        args.push(format!("-NewName '{}'", escape_ps_string(&input.new_name)));
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name, Id, \
             @{{N='SwitchType';E={{$_.SwitchType.ToString()}}}}, \
             @{{N='Status';E={{$_.Status.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let switches = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(switches.len());
        for sw in switches {
            output.push(VMSwitchInfo {
                name: sw["Name"].as_str().unwrap_or_default().to_string(),
                id: sw["Id"].as_str().unwrap_or_default().to_string(),
                switch_type: sw["SwitchType"].as_str().unwrap_or_default().to_string(),
                status: sw["Status"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(RenameVmSwitchOutput { switches: output })
    }
}

register_tool!(RenameVmSwitchTool);
