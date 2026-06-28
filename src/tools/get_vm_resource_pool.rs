use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmResourcePoolInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "resourcePoolType")]
    pub resource_pool_type: String,
    #[serde(rename = "parentName")]
    pub parent_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmResourcePoolInput {
    /// Name of the resource pool.
    #[serde(default, rename = "name")]
    pub name: Option<String>,
    /// Type of the resource pool.
    #[serde(default, rename = "resourcePoolType")]
    pub resource_pool_type: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmResourcePoolOutput {
    pub pools: Vec<VmResourcePoolInfo>,
}


#[derive(Default)]
pub struct GetVmResourcePoolTool;

#[async_trait]
impl HyperVTool for GetVmResourcePoolTool {
    const NAME: &'static str = "hyperv_get_vm_resource_pool";
    const DESCRIPTION: &'static str = "Gets the resource pools on one or more virtual machine hosts.";
    type Input = GetVmResourcePoolInput;
    type Output = GetVmResourcePoolOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMResourcePool".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput("name must not be empty when provided".to_string()));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(resource_pool_type) = &input.resource_pool_type {
            if resource_pool_type.trim().is_empty() {
                return Err(ToolError::InvalidInput("resource_pool_type must not be empty when provided".to_string()));
            }
            args.push(format!("-ResourcePoolType '{}'", escape_ps_string(resource_pool_type)));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, @{{N='ResourcePoolType';E={{$_.ResourcePoolType.ToString()}}}}, ParentName, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmResourcePoolInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                resource_pool_type: item["ResourcePoolType"].as_str().unwrap_or_default().to_string(),
                parent_name: item["ParentName"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(GetVmResourcePoolOutput { pools: output })

    }
}


register_tool!(GetVmResourcePoolTool);
