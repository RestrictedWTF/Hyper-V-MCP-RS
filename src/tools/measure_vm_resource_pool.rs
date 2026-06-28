use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmResourcePoolMeasuredInfo {
    pub name: String,
    #[serde(rename = "resourcePoolType")]
    pub resource_pool_type: String,
    #[serde(rename = "avgCPU")]
    pub avg_cpu: f64,
    #[serde(rename = "avgRAM")]
    pub avg_ram: f64,
    #[serde(rename = "maxRAM")]
    pub max_ram: f64,
    #[serde(rename = "minRAM")]
    pub min_ram: f64,
    #[serde(rename = "totalDisk")]
    pub total_disk: f64,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MeasureVmResourcePoolInput {
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
pub struct MeasureVmResourcePoolOutput {
    pub measurements: Vec<VmResourcePoolMeasuredInfo>,
}

#[derive(Default)]
pub struct MeasureVmResourcePoolTool;

#[async_trait]
impl HyperVTool for MeasureVmResourcePoolTool {
    const NAME: &'static str = "hyperv_measure_vm_resource_pool";
    const DESCRIPTION: &'static str =
        "Reports resource utilization data for one or more resource pools.";
    type Input = MeasureVmResourcePoolInput;
    type Output = MeasureVmResourcePoolOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Measure-VMResourcePool".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(resource_pool_type) = &input.resource_pool_type {
            if resource_pool_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "resource_pool_type must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ResourcePoolType '{}'",
                escape_ps_string(resource_pool_type)
            ));
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

        let ps = format!("{} | Select-Object Name, @{{N='ResourcePoolType';E={{$_.ResourcePoolType.ToString()}}}}, AvgCPU, AvgRAM, MaxRAM, MinRAM, TotalDisk, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmResourcePoolMeasuredInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                resource_pool_type: item["ResourcePoolType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                avg_cpu: item["AvgCPU"].as_f64().unwrap_or_default(),
                avg_ram: item["AvgRAM"].as_f64().unwrap_or_default(),
                max_ram: item["MaxRAM"].as_f64().unwrap_or_default(),
                min_ram: item["MinRAM"].as_f64().unwrap_or_default(),
                total_disk: item["TotalDisk"].as_f64().unwrap_or_default(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(MeasureVmResourcePoolOutput {
            measurements: output,
        })
    }
}

register_tool!(MeasureVmResourcePoolTool);
