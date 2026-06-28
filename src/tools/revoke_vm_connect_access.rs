use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RevokeVmConnectAccessInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// User name to revoke access.
    #[serde(rename = "userName")]
    pub user_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RevokeVmConnectAccessOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RevokeVmConnectAccessTool;

#[async_trait]
impl HyperVTool for RevokeVmConnectAccessTool {
    const NAME: &'static str = "hyperv_revoke_vm_connect_access";
    const DESCRIPTION: &'static str = "Revokes access for one or more users to connect to a one or more virtual machines.";
    type Input = RevokeVmConnectAccessInput;
    type Output = RevokeVmConnectAccessOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Revoke-VMConnectAccess".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if input.user_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("user_name must not be empty".to_string()));
        }
        args.push(format!("-UserName '{}'", escape_ps_string(&input.user_name)));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = args.join(" ");

        ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        Ok(RevokeVmConnectAccessOutput { success: true })
    }
}


register_tool!(RevokeVmConnectAccessTool);
