use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmGroupMemberInput {
    /// Name of the virtual machine group.
    pub name: String,
    /// Names of virtual machines to remove.
    #[serde(default, rename = "vmNames")]
    pub vm_names: Option<Vec<String>>,
    /// Names of VM groups to remove.
    #[serde(default, rename = "vmGroupNames")]
    pub vm_group_names: Option<Vec<String>>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmGroupMemberOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmGroupMemberTool;

#[async_trait]
impl HyperVTool for RemoveVmGroupMemberTool {
    const NAME: &'static str = "hyperv_remove_vm_group_member";
    const DESCRIPTION: &'static str = "Removes members from a virtual machine group.";
    type Input = RemoveVmGroupMemberInput;
    type Output = RemoveVmGroupMemberOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMGroupMember".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "name must not be empty".to_string(),
            ));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if let Some(vm_names) = &input.vm_names {
            let escaped: Vec<String> = vm_names
                .iter()
                .map(|n| format!("'{}'", escape_ps_string(n)))
                .collect();
            args.push(format!("-VM {}", escaped.join(",")));
        }
        if let Some(vm_group_names) = &input.vm_group_names {
            let escaped: Vec<String> = vm_group_names
                .iter()
                .map(|n| format!("'{}'", escape_ps_string(n)))
                .collect();
            args.push(format!("-VMGroupMember {}", escaped.join(",")));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = args.join(" ");

        ctx.sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(RemoveVmGroupMemberOutput { success: true })
    }
}

register_tool!(RemoveVmGroupMemberTool);
