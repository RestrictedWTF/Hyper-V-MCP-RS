use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostPartitionableGpuInput {
    /// Name of the partitionable GPU to retrieve. If omitted, returns all partitionable GPUs.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostPartitionableGpuInfo {
    pub name: String,
    #[serde(rename = "validPartitionCounts")]
    pub valid_partition_counts: Vec<u64>,
    #[serde(rename = "partitionCount")]
    pub partition_count: u64,
    #[serde(rename = "totalVRAM")]
    pub total_vram: u64,
    #[serde(rename = "availableVRAM")]
    pub available_vram: u64,
    #[serde(rename = "minPartitionVRAM")]
    pub min_partition_vram: u64,
    #[serde(rename = "maxPartitionVRAM")]
    pub max_partition_vram: u64,
    #[serde(rename = "optimalPartitionVRAM")]
    pub optimal_partition_vram: u64,
    #[serde(rename = "totalEncode")]
    pub total_encode: u64,
    #[serde(rename = "availableEncode")]
    pub available_encode: u64,
    #[serde(rename = "minPartitionEncode")]
    pub min_partition_encode: u64,
    #[serde(rename = "maxPartitionEncode")]
    pub max_partition_encode: u64,
    #[serde(rename = "optimalPartitionEncode")]
    pub optimal_partition_encode: u64,
    #[serde(rename = "totalDecode")]
    pub total_decode: u64,
    #[serde(rename = "availableDecode")]
    pub available_decode: u64,
    #[serde(rename = "minPartitionDecode")]
    pub min_partition_decode: u64,
    #[serde(rename = "maxPartitionDecode")]
    pub max_partition_decode: u64,
    #[serde(rename = "optimalPartitionDecode")]
    pub optimal_partition_decode: u64,
    #[serde(rename = "totalCompute")]
    pub total_compute: u64,
    #[serde(rename = "availableCompute")]
    pub available_compute: u64,
    #[serde(rename = "minPartitionCompute")]
    pub min_partition_compute: u64,
    #[serde(rename = "maxPartitionCompute")]
    pub max_partition_compute: u64,
    #[serde(rename = "optimalPartitionCompute")]
    pub optimal_partition_compute: u64,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostPartitionableGpuOutput {
    pub gpus: Vec<VmHostPartitionableGpuInfo>,
}

#[derive(Default)]
pub struct GetVmHostPartitionableGpuTool;

fn u64s_from(value: &serde_json::Value) -> Vec<u64> {
    match value {
        serde_json::Value::Array(arr) => {
            arr.iter().map(|v| v.as_u64().unwrap_or_default()).collect()
        }
        serde_json::Value::Number(n) => vec![n.as_u64().unwrap_or_default()],
        _ => Vec::new(),
    }
}

#[async_trait]
impl HyperVTool for GetVmHostPartitionableGpuTool {
    const NAME: &'static str = "hyperv_get_vm_host_partitionable_gpu";
    const DESCRIPTION: &'static str = "Gets the host machine's partitionable GPU.";
    type Input = GetVmHostPartitionableGpuInput;
    type Output = GetVmHostPartitionableGpuOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHostPartitionableGpu".to_string()];

        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "GPU name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, ValidPartitionCounts, PartitionCount, \
             TotalVRAM, AvailableVRAM, MinPartitionVRAM, MaxPartitionVRAM, OptimalPartitionVRAM, \
             TotalEncode, AvailableEncode, MinPartitionEncode, MaxPartitionEncode, OptimalPartitionEncode, \
             TotalDecode, AvailableDecode, MinPartitionDecode, MaxPartitionDecode, OptimalPartitionDecode, \
             TotalCompute, AvailableCompute, MinPartitionCompute, MaxPartitionCompute, OptimalPartitionCompute, \
             ComputerName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let gpus = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(gpus.len());
        for gpu in gpus {
            output.push(VmHostPartitionableGpuInfo {
                name: gpu["Name"].as_str().unwrap_or_default().to_string(),
                valid_partition_counts: u64s_from(&gpu["ValidPartitionCounts"]),
                partition_count: gpu["PartitionCount"].as_u64().unwrap_or_default(),
                total_vram: gpu["TotalVRAM"].as_u64().unwrap_or_default(),
                available_vram: gpu["AvailableVRAM"].as_u64().unwrap_or_default(),
                min_partition_vram: gpu["MinPartitionVRAM"].as_u64().unwrap_or_default(),
                max_partition_vram: gpu["MaxPartitionVRAM"].as_u64().unwrap_or_default(),
                optimal_partition_vram: gpu["OptimalPartitionVRAM"].as_u64().unwrap_or_default(),
                total_encode: gpu["TotalEncode"].as_u64().unwrap_or_default(),
                available_encode: gpu["AvailableEncode"].as_u64().unwrap_or_default(),
                min_partition_encode: gpu["MinPartitionEncode"].as_u64().unwrap_or_default(),
                max_partition_encode: gpu["MaxPartitionEncode"].as_u64().unwrap_or_default(),
                optimal_partition_encode: gpu["OptimalPartitionEncode"]
                    .as_u64()
                    .unwrap_or_default(),
                total_decode: gpu["TotalDecode"].as_u64().unwrap_or_default(),
                available_decode: gpu["AvailableDecode"].as_u64().unwrap_or_default(),
                min_partition_decode: gpu["MinPartitionDecode"].as_u64().unwrap_or_default(),
                max_partition_decode: gpu["MaxPartitionDecode"].as_u64().unwrap_or_default(),
                optimal_partition_decode: gpu["OptimalPartitionDecode"]
                    .as_u64()
                    .unwrap_or_default(),
                total_compute: gpu["TotalCompute"].as_u64().unwrap_or_default(),
                available_compute: gpu["AvailableCompute"].as_u64().unwrap_or_default(),
                min_partition_compute: gpu["MinPartitionCompute"].as_u64().unwrap_or_default(),
                max_partition_compute: gpu["MaxPartitionCompute"].as_u64().unwrap_or_default(),
                optimal_partition_compute: gpu["OptimalPartitionCompute"]
                    .as_u64()
                    .unwrap_or_default(),
                computer_name: gpu["ComputerName"].as_str().unwrap_or_default().to_string(),
                is_deleted: gpu["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmHostPartitionableGpuOutput { gpus: output })
    }
}

register_tool!(GetVmHostPartitionableGpuTool);
