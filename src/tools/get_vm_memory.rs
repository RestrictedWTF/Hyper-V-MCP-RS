use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmMemoryInput {
    /// Name of the virtual machine. If omitted, returns memory for all VMs.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Name of the checkpoint/snapshot to retrieve memory for. Requires vmName.
    #[serde(default, rename = "snapshotName")]
    pub snapshot_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmMemoryInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmSnapshotName")]
    pub vm_snapshot_name: String,
    pub id: String,
    pub dynamic_memory_enabled: bool,
    pub startup: u64,
    pub minimum: u64,
    pub maximum: u64,
    #[serde(rename = "maximumPerNumaNode")]
    pub maximum_per_numa_node: u64,
    pub buffer: u32,
    pub priority: u32,
    #[serde(rename = "resourcePoolName")]
    pub resource_pool_name: String,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmMemoryOutput {
    pub memories: Vec<VmMemoryInfo>,
}

#[derive(Default)]
pub struct GetVmMemoryTool;

#[async_trait]
impl HyperVTool for GetVmMemoryTool {
    const NAME: &'static str = "hyperv_get_vm_memory";
    const DESCRIPTION: &'static str = "Gets the memory of a virtual machine or snapshot.";
    type Input = GetVmMemoryInput;
    type Output = GetVmMemoryOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let ps = if let Some(snapshot_name) = &input.snapshot_name {
            let vm_name = input.vm_name.as_ref().ok_or_else(|| {
                ToolError::InvalidInput(
                    "vmName is required when snapshotName is provided".to_string(),
                )
            })?;
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            if snapshot_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Snapshot name must not be empty".to_string(),
                ));
            }

            let mut cmd = format!(
                "Get-VMSnapshot -VMName '{}' -Name '{}'",
                escape_ps_string(vm_name),
                escape_ps_string(snapshot_name)
            );
            if let Some(computer) = &input.computer_name {
                cmd.push_str(&format!(" -ComputerName '{}'", escape_ps_string(computer)));
            }
            format!(
                "{} | Get-VMMemory | Select-Object VMName, VMSnapshotName, Id, DynamicMemoryEnabled, Startup, Minimum, Maximum, MaximumPerNumaNode, Buffer, Priority, ResourcePoolName, ComputerName | ConvertTo-Json -Compress -Depth 3",
                cmd
            )
        } else if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }

            let mut args = vec![format!(
                "Get-VMMemory -VMName '{}'",
                escape_ps_string(vm_name)
            )];
            if let Some(computer) = &input.computer_name {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
            format!(
                "{} | Select-Object VMName, VMSnapshotName, Id, DynamicMemoryEnabled, Startup, Minimum, Maximum, MaximumPerNumaNode, Buffer, Priority, ResourcePoolName, ComputerName | ConvertTo-Json -Compress -Depth 3",
                args.join(" ")
            )
        } else {
            let mut args = vec!["Get-VM".to_string()];
            if let Some(computer) = &input.computer_name {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
            format!(
                "{} | Get-VMMemory | Select-Object VMName, VMSnapshotName, Id, DynamicMemoryEnabled, Startup, Minimum, Maximum, MaximumPerNumaNode, Buffer, Priority, ResourcePoolName, ComputerName | ConvertTo-Json -Compress -Depth 3",
                args.join(" ")
            )
        };

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let memories = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(memories.len());
        for mem in memories {
            output.push(VmMemoryInfo {
                vm_name: mem["VMName"].as_str().unwrap_or_default().to_string(),
                vm_snapshot_name: mem["VMSnapshotName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                id: mem["Id"].as_str().unwrap_or_default().to_string(),
                dynamic_memory_enabled: mem["DynamicMemoryEnabled"].as_bool().unwrap_or_default(),
                startup: mem["Startup"].as_u64().unwrap_or_default(),
                minimum: mem["Minimum"].as_u64().unwrap_or_default(),
                maximum: mem["Maximum"].as_u64().unwrap_or_default(),
                maximum_per_numa_node: mem["MaximumPerNumaNode"].as_u64().unwrap_or_default(),
                buffer: mem["Buffer"].as_u64().unwrap_or_default() as u32,
                priority: mem["Priority"].as_u64().unwrap_or_default() as u32,
                resource_pool_name: mem["ResourcePoolName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: mem["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(GetVmMemoryOutput { memories: output })
    }
}

register_tool!(GetVmMemoryTool);
