use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmStoragePathInput {
    /// Name of the storage resource pool.
pub name: String,
    /// Path to remove.
    #[serde(rename = "path")]
    pub path: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}
#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmStoragePathOutput {
    pub success: bool,
}

#[derive(Default)]
pub struct RemoveVmStoragePathTool;

#[async_trait]
impl HyperVTool for RemoveVmStoragePathTool {
    const NAME: &'static str = "hyperv_remove_vm_storage_path";
    const DESCRIPTION: &'static str = "Removes a path from a storage resource pool.";
    type Input = RemoveVmStoragePathInput;
    type Output = RemoveVmStoragePathOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Remove-VMStoragePath".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput("path must not be empty".to_string()));
        }
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
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

        Ok(RemoveVmStoragePathOutput { success: true })
    }
}


register_tool!(RemoveVmStoragePathTool);
