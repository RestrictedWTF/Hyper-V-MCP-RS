use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmStoragePathInput {
    /// Path to be added to the storage resource pool.
    pub path: String,
    /// Name of the resource pool to which the path is to be added.
    #[serde(rename = "resourcePoolName")]
    pub resource_pool_name: String,
    /// Type of the resource pool. Allowed values: VHD, ISO, VFD.
    #[serde(rename = "resourcePoolType")]
    pub resource_pool_type: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmStoragePathOutput {
    pub success: bool,
    #[serde(rename = "addedPaths")]
    pub added_paths: Vec<String>,
}

#[derive(Default)]
pub struct AddVmStoragePathTool;

#[async_trait]
impl HyperVTool for AddVmStoragePathTool {
    const NAME: &'static str = "hyperv_add_vm_storage_path";
    const DESCRIPTION: &'static str = "Adds a path to a storage resource pool.";
    type Input = AddVmStoragePathInput;
    type Output = AddVmStoragePathOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }
        if input.resource_pool_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "ResourcePoolName must not be empty".to_string(),
            ));
        }

        let resource_pool_type = input.resource_pool_type.to_uppercase();
        if !matches!(resource_pool_type.as_str(), "VHD" | "ISO" | "VFD") {
            return Err(ToolError::InvalidInput(
                "ResourcePoolType must be one of: VHD, ISO, VFD".to_string(),
            ));
        }

        let mut args = vec!["Add-VMStoragePath".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
        args.push(format!(
            "-ResourcePoolName '{}'",
            escape_ps_string(&input.resource_pool_name)
        ));
        args.push(format!(
            "-ResourcePoolType '{}'",
            escape_ps_string(&resource_pool_type)
        ));
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-PassThru".to_string());

        let ps = format!("{} | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let added_paths = match raw {
            serde_json::Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            serde_json::Value::String(s) => vec![s],
            _ => Vec::new(),
        };

        Ok(AddVmStoragePathOutput {
            success: true,
            added_paths,
        })
    }
}

register_tool!(AddVmStoragePathTool);
