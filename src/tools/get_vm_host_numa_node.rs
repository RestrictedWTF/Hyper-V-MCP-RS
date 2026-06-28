use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostNumaNodeInput {
    /// NUMA node identifier to retrieve. If omitted, returns all NUMA nodes.
    #[serde(default)]
    pub id: Option<i32>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostNumaNodeInfo {
    #[serde(rename = "nodeId")]
    pub node_id: i32,
    #[serde(rename = "processorsAvailability")]
    pub processors_availability: Vec<u32>,
    #[serde(rename = "memoryAvailable")]
    pub memory_available: u64,
    #[serde(rename = "memoryTotal")]
    pub memory_total: u64,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostNumaNodeOutput {
    pub nodes: Vec<VmHostNumaNodeInfo>,
}

#[derive(Default)]
pub struct GetVmHostNumaNodeTool;

#[async_trait]
impl HyperVTool for GetVmHostNumaNodeTool {
    const NAME: &'static str = "hyperv_get_vm_host_numa_node";
    const DESCRIPTION: &'static str = "Gets the NUMA topology of a virtual machine host.";
    type Input = GetVmHostNumaNodeInput;
    type Output = GetVmHostNumaNodeOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHostNumaNode".to_string()];

        if let Some(id) = &input.id {
            args.push(format!("-Id {}", id));
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
             NodeId, ProcessorsAvailability, MemoryAvailable, MemoryTotal, ComputerName | \
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

        let nodes = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(nodes.len());
        for node in nodes {
            let processors_availability = match &node["ProcessorsAvailability"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_u64().unwrap_or_default() as u32)
                    .collect(),
                _ => Vec::new(),
            };

            output.push(VmHostNumaNodeInfo {
                node_id: node["NodeId"].as_i64().unwrap_or_default() as i32,
                processors_availability,
                memory_available: node["MemoryAvailable"].as_u64().unwrap_or_default(),
                memory_total: node["MemoryTotal"].as_u64().unwrap_or_default(),
                computer_name: node["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmHostNumaNodeOutput { nodes: output })
    }
}

register_tool!(GetVmHostNumaNodeTool);
