use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmStoragePathInfo {
    pub name: String,
    pub path: String,
    #[serde(rename = "poolName")]
    pub pool_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmStoragePathInput {
    /// Name of the storage resource pool.
    #[serde(default, rename = "name")]
    pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmStoragePathOutput {
    pub paths: Vec<VmStoragePathInfo>,
}

#[derive(Default)]
pub struct GetVmStoragePathTool;

#[async_trait]
impl HyperVTool for GetVmStoragePathTool {
    const NAME: &'static str = "hyperv_get_vm_storage_path";
    const DESCRIPTION: &'static str = "Gets the storage paths in a storage resource pool.";
    type Input = GetVmStoragePathInput;
    type Output = GetVmStoragePathOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMStoragePath".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
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

        let ps = format!("{} | Select-Object Name, Path, PoolName, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            output.push(VmStoragePathInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                path: item["Path"].as_str().unwrap_or_default().to_string(),
                pool_name: item["PoolName"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmStoragePathOutput { paths: output })
    }
}

register_tool!(GetVmStoragePathTool);
