use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmRemoteFxPhysicalVideoAdapterInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "gpuName")]
    pub gpu_name: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmRemoteFxPhysicalVideoAdapterInput {
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmRemoteFxPhysicalVideoAdapterOutput {
    pub adapters: Vec<VmRemoteFxPhysicalVideoAdapterInfo>,
}


#[derive(Default)]
pub struct GetVmRemoteFxPhysicalVideoAdapterTool;

#[async_trait]
impl HyperVTool for GetVmRemoteFxPhysicalVideoAdapterTool {
    const NAME: &'static str = "hyperv_get_vm_remote_fx_physical_video_adapter";
    const DESCRIPTION: &'static str = "Gets the RemoteFX physical graphics adapters on one or more Hyper-V hosts.";
    type Input = GetVmRemoteFxPhysicalVideoAdapterInput;
    type Output = GetVmRemoteFxPhysicalVideoAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMRemoteFXPhysicalVideoAdapter".to_string()];
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, GpuName, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmRemoteFxPhysicalVideoAdapterInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                gpu_name: item["GpuName"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(GetVmRemoteFxPhysicalVideoAdapterOutput { adapters: output })

    }
}


register_tool!(GetVmRemoteFxPhysicalVideoAdapterTool);
