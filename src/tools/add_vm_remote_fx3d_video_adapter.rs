use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmRemoteFx3dVideoAdapterAddInfo {
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
pub struct AddVmRemoteFx3dVideoAdapterInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Number of monitors.
    #[serde(default, rename = "monitorCount")]
    pub monitor_count: Option<u32>,
    /// Maximum resolution.
    #[serde(default, rename = "maximumResolution")]
    pub maximum_resolution: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmRemoteFx3dVideoAdapterOutput {
    pub adapters: Vec<VmRemoteFx3dVideoAdapterAddInfo>,
}


#[derive(Default)]
pub struct AddVmRemoteFx3dVideoAdapterTool;

#[async_trait]
impl HyperVTool for AddVmRemoteFx3dVideoAdapterTool {
    const NAME: &'static str = "hyperv_add_vm_remote_fx3d_video_adapter";
    const DESCRIPTION: &'static str = "Adds a RemoteFX video adapter in a virtual machine.";
    type Input = AddVmRemoteFx3dVideoAdapterInput;
    type Output = AddVmRemoteFx3dVideoAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Add-VMRemoteFx3dVideoAdapter".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(monitor_count) = &input.monitor_count {
            args.push(format!("-MonitorCount {}", monitor_count));
        }
        if let Some(maximum_resolution) = &input.maximum_resolution {
            if maximum_resolution.trim().is_empty() {
                return Err(ToolError::InvalidInput("maximum_resolution must not be empty when provided".to_string()));
            }
            args.push(format!("-MaximumResolution '{}'", escape_ps_string(maximum_resolution)));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        args.push("-PassThru".to_string());
        let ps = format!("{} | Select-Object Name, Id, VMName, VMId, MonitorCount, MaximumResolution | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmRemoteFx3dVideoAdapterAddInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                monitor_count: item["MonitorCount"].as_u64().unwrap_or_default() as u32,
                maximum_resolution: item["MaximumResolution"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(AddVmRemoteFx3dVideoAdapterOutput { adapters: output })

    }
}


register_tool!(AddVmRemoteFx3dVideoAdapterTool);
