use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostNumaNodeStatusInput {
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Identifies a NUMA node for which virtual machine status is to be retrieved.
    #[serde(default)]
    pub id: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostNumaNodeStatus {
    pub node_id: i32,
    pub vm_id: String,
    pub vm_name: String,
    pub memory_used: i32,
    pub computer_name: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostNumaNodeStatusOutput {
    pub nodes: Vec<VmHostNumaNodeStatus>,
}

#[derive(Default)]
pub struct GetVmHostNumaNodeStatusTool;

#[async_trait]
impl HyperVTool for GetVmHostNumaNodeStatusTool {
    const NAME: &'static str = "hyperv_get_vm_host_numa_node_status";
    const DESCRIPTION: &'static str =
        "Gets the status of the virtual machines on the non-uniform memory access (NUMA) nodes of a virtual machine host or hosts.";
    type Input = GetVmHostNumaNodeStatusInput;
    type Output = GetVmHostNumaNodeStatusOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHostNumaNodeStatus".to_string()];
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(id) = input.id {
            args.push(format!("-Id {}", id));
        }

        let ps = format!(
            "{} | Select-Object NodeId, \
             @{{N='VMId';E={{$_.VMId.ToString()}}}}, \
             VMName, MemoryUsed, ComputerName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
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
            output.push(VmHostNumaNodeStatus {
                node_id: node["NodeId"].as_i64().unwrap_or_default() as i32,
                vm_id: node["VMId"].as_str().unwrap_or_default().to_string(),
                vm_name: node["VMName"].as_str().unwrap_or_default().to_string(),
                memory_used: node["MemoryUsed"].as_i64().unwrap_or_default() as i32,
                computer_name: node["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: node["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmHostNumaNodeStatusOutput { nodes: output })
    }
}

register_tool!(GetVmHostNumaNodeStatusTool);
