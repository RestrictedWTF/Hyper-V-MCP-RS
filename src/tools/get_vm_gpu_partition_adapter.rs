use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmGpuPartitionAdapterInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "minPartitionVRAM")]
    pub min_partition_vram: u64,
    #[serde(rename = "maxPartitionVRAM")]
    pub max_partition_vram: u64,
    #[serde(rename = "optimalPartitionVRAM")]
    pub optimal_partition_vram: u64,
    #[serde(rename = "totalVRAM")]
    pub total_vram: u64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmGpuPartitionAdapterInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmGpuPartitionAdapterOutput {
    pub adapters: Vec<VmGpuPartitionAdapterInfo>,
}

#[derive(Default)]
pub struct GetVmGpuPartitionAdapterTool;

#[async_trait]
impl HyperVTool for GetVmGpuPartitionAdapterTool {
    const NAME: &'static str = "hyperv_get_vm_gpu_partition_adapter";
    const DESCRIPTION: &'static str =
        "Gets the information of assigned GPU partitions to a virtual machine.";
    type Input = GetVmGpuPartitionAdapterInput;
    type Output = GetVmGpuPartitionAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMGpuPartitionAdapter".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
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

        let ps = format!("{} | Select-Object Name, Id, VMName, InstancePath, MinPartitionVRAM, MaxPartitionVRAM, OptimalPartitionVRAM, TotalVRAM | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmGpuPartitionAdapterInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                instance_path: item["InstancePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                min_partition_vram: item["MinPartitionVRAM"].as_u64().unwrap_or_default(),
                max_partition_vram: item["MaxPartitionVRAM"].as_u64().unwrap_or_default(),
                optimal_partition_vram: item["OptimalPartitionVRAM"].as_u64().unwrap_or_default(),
                total_vram: item["TotalVRAM"].as_u64().unwrap_or_default(),
            });
        }

        Ok(GetVmGpuPartitionAdapterOutput { adapters: output })
    }
}

register_tool!(GetVmGpuPartitionAdapterTool);
