use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmMeasuredResourceInfo {
    pub name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
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
    #[serde(rename = "networkInbound")]
    pub network_inbound: f64,
    #[serde(rename = "networkOutbound")]
    pub network_outbound: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MeasureVmInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct MeasureVmOutput {
    pub measurements: Vec<VmMeasuredResourceInfo>,
}

#[derive(Default)]
pub struct MeasureVmTool;

#[async_trait]
impl HyperVTool for MeasureVmTool {
    const NAME: &'static str = "hyperv_measure_vm";
    const DESCRIPTION: &'static str =
        "Reports resource utilization data for one or more virtual machines.";
    type Input = MeasureVmInput;
    type Output = MeasureVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Measure-VM".to_string()];
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

        let ps = format!("{} | Select-Object Name, VMId, AvgCPU, AvgRAM, MaxRAM, MinRAM, TotalDisk, NetworkInbound, NetworkOutbound | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmMeasuredResourceInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                avg_cpu: item["AvgCPU"].as_f64().unwrap_or_default(),
                avg_ram: item["AvgRAM"].as_f64().unwrap_or_default(),
                max_ram: item["MaxRAM"].as_f64().unwrap_or_default(),
                min_ram: item["MinRAM"].as_f64().unwrap_or_default(),
                total_disk: item["TotalDisk"].as_f64().unwrap_or_default(),
                network_inbound: item["NetworkInbound"].as_f64().unwrap_or_default(),
                network_outbound: item["NetworkOutbound"].as_f64().unwrap_or_default(),
            });
        }

        Ok(MeasureVmOutput {
            measurements: output,
        })
    }
}

register_tool!(MeasureVmTool);
