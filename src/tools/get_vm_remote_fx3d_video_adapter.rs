use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmRemoteFx3dVideoAdapterInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "monitorCount")]
    pub monitor_count: u32,
    #[serde(rename = "maximumResolution")]
    pub maximum_resolution: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmRemoteFx3dVideoAdapterInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmRemoteFx3dVideoAdapterOutput {
    pub adapters: Vec<VmRemoteFx3dVideoAdapterInfo>,
}


#[derive(Default)]
pub struct GetVmRemoteFx3dVideoAdapterTool;

#[async_trait]
impl HyperVTool for GetVmRemoteFx3dVideoAdapterTool {
    const NAME: &'static str = "hyperv_get_vm_remote_fx3d_video_adapter";
    const DESCRIPTION: &'static str = "Gets the RemoteFX video adapter of a virtual machine or snapshot.";
    type Input = GetVmRemoteFx3dVideoAdapterInput;
    type Output = GetVmRemoteFx3dVideoAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMRemoteFx3dVideoAdapter".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("vm_name must not be empty when provided".to_string()));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, VMName, VMId, MonitorCount, @{{N='MaximumResolution';E={{$_.MaximumResolution.ToString()}}}} | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmRemoteFx3dVideoAdapterInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                monitor_count: item["MonitorCount"].as_u64().unwrap_or_default() as u32,
                maximum_resolution: item["MaximumResolution"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(GetVmRemoteFx3dVideoAdapterOutput { adapters: output })

    }
}


register_tool!(GetVmRemoteFx3dVideoAdapterTool);
