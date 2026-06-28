use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmHostPartitionableGpuInput {
    /// Hyper-V host on which the GPU resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Name of the GPU to configure.
    #[serde(default)]
    pub name: Option<String>,
    /// Number of partitions that the GPU will assign. The number of partitions
    /// is defined by the GPU manufacturer.
    #[serde(default, rename = "partitionCount")]
    pub partition_count: Option<u16>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostPartitionableGpuInfo {
    pub name: String,
    #[serde(rename = "instancePath")]
    pub instance_path: String,
    #[serde(rename = "partitionCount")]
    pub partition_count: u16,
    #[serde(rename = "totalPartitionCount")]
    pub total_partition_count: u16,
    #[serde(rename = "validPartitionCounts")]
    pub valid_partition_counts: Vec<u16>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmHostPartitionableGpuOutput {
    pub gpus: Vec<VmHostPartitionableGpuInfo>,
}

#[derive(Default)]
pub struct SetVmHostPartitionableGpuTool;

#[async_trait]
impl HyperVTool for SetVmHostPartitionableGpuTool {
    const NAME: &'static str = "hyperv_set_vm_host_partitionable_gpu";
    const DESCRIPTION: &'static str =
        "Configures the host partitionable GPU to the number of partitions supported by the manufacturer.";
    type Input = SetVmHostPartitionableGpuInput;
    type Output = SetVmHostPartitionableGpuOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "GPU name must not be empty".to_string(),
                ));
            }
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
        }

        let mut args = vec!["Set-VMHostPartitionableGpu".to_string()];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(name) = &input.name {
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(count) = input.partition_count {
            args.push(format!("-PartitionCount {}", count));
        }


        let ps = format!(
            "{} | Select-Object \
             Name, InstancePath, PartitionCount, TotalPartitionCount, ValidPartitionCounts | \
             ConvertTo-Json -Compress -Depth 3",
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
                instance_path: gpu["InstancePath"].as_str().unwrap_or_default().to_string(),
                partition_count: gpu["PartitionCount"].as_u64().unwrap_or_default() as u16,
                total_partition_count: gpu["TotalPartitionCount"].as_u64().unwrap_or_default()
                    as u16,
                valid_partition_counts: gpu["ValidPartitionCounts"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_u64().map(|n| n as u16))
                            .collect()
                    })
                    .unwrap_or_default(),
            });
        }

        Ok(SetVmHostPartitionableGpuOutput { gpus: output })
    }
}

register_tool!(SetVmHostPartitionableGpuTool);
