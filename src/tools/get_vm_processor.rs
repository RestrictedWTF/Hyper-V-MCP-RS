use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmProcessorInput {
    /// Name of the virtual machine whose processor is to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Name of the snapshot whose processor is to be retrieved.
    #[serde(default, rename = "snapshotName")]
    pub snapshot_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmProcessorDetails {
    pub vm_name: String,
    pub vm_id: String,
    pub snapshot_name: String,
    pub snapshot_id: String,
    pub count: i64,
    pub compatibility_for_migration_enabled: bool,
    pub compatibility_for_older_operating_systems_enabled: bool,
    pub hw_thread_count_per_core: i64,
    pub expose_virtualization_extensions: bool,
    pub maximum: i64,
    pub reserve: i64,
    pub relative_weight: i32,
    pub maximum_count_per_numa_node: i64,
    pub maximum_count_per_numa_socket: i64,
    pub resource_pool_name: String,
    pub enable_host_resource_protection: bool,
    pub computer_name: String,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmProcessorOutput {
    pub processors: Vec<VmProcessorDetails>,
}

#[derive(Default)]
pub struct GetVmProcessorTool;

#[async_trait]
impl HyperVTool for GetVmProcessorTool {
    const NAME: &'static str = "hyperv_get_vm_processor";
    const DESCRIPTION: &'static str = "Gets the processor of a virtual machine or snapshot.";
    type Input = GetVmProcessorInput;
    type Output = GetVmProcessorOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
        }
        if let Some(snapshot_name) = &input.snapshot_name {
            if snapshot_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Snapshot name must not be empty".to_string(),
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

        let select = "Select-Object \
            VMName, VMId, VMSnapshotId, VMSnapshotName, Count, \
            CompatibilityForMigrationEnabled, CompatibilityForOlderOperatingSystemsEnabled, \
            HwThreadCountPerCore, ExposeVirtualizationExtensions, Maximum, Reserve, RelativeWeight, \
            MaximumCountPerNumaNode, MaximumCountPerNumaSocket, ResourcePoolName, \
            EnableHostResourceProtection, ComputerName, IsDeleted";

        let ps = if let Some(snapshot_name) = &input.snapshot_name {
            let mut snap_args = vec!["Get-VMSnapshot".to_string()];
            if let Some(vm_name) = &input.vm_name {
                snap_args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
            }
            if let Some(computer) = &input.computer_name {
                snap_args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
            snap_args.push(format!("-Name '{}'", escape_ps_string(snapshot_name)));
            format!(
                "{} | Get-VMProcessor | {} | ConvertTo-Json -Compress -Depth 3",
                snap_args.join(" "),
                select
            )
        } else if let Some(vm_name) = &input.vm_name {
            let mut args = vec![format!(
                "Get-VMProcessor -VMName '{}'",
                escape_ps_string(vm_name)
            )];
            if let Some(computer) = &input.computer_name {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }
            format!(
                "{} | {} | ConvertTo-Json -Compress -Depth 3",
                args.join(" "),
                select
            )
        } else {
            return Err(ToolError::InvalidInput(
                "vm_name or snapshot_name must be provided".to_string(),
            ));
        };

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let processors = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(processors.len());
        for proc in processors {
            output.push(VmProcessorDetails {
                vm_name: proc["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: proc["VMId"].as_str().unwrap_or_default().to_string(),
                snapshot_name: proc["VMSnapshotName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                snapshot_id: proc["VMSnapshotId"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                count: proc["Count"].as_i64().unwrap_or_default(),
                compatibility_for_migration_enabled: proc["CompatibilityForMigrationEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
                compatibility_for_older_operating_systems_enabled: proc
                    ["CompatibilityForOlderOperatingSystemsEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
                hw_thread_count_per_core: proc["HwThreadCountPerCore"].as_i64().unwrap_or_default(),
                expose_virtualization_extensions: proc["ExposeVirtualizationExtensions"]
                    .as_bool()
                    .unwrap_or_default(),
                maximum: proc["Maximum"].as_i64().unwrap_or_default(),
                reserve: proc["Reserve"].as_i64().unwrap_or_default(),
                relative_weight: proc["RelativeWeight"].as_i64().unwrap_or_default() as i32,
                maximum_count_per_numa_node: proc["MaximumCountPerNumaNode"]
                    .as_i64()
                    .unwrap_or_default(),
                maximum_count_per_numa_socket: proc["MaximumCountPerNumaSocket"]
                    .as_i64()
                    .unwrap_or_default(),
                resource_pool_name: proc["ResourcePoolName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                enable_host_resource_protection: proc["EnableHostResourceProtection"]
                    .as_bool()
                    .unwrap_or_default(),
                computer_name: proc["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: proc["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmProcessorOutput { processors: output })
    }
}

register_tool!(GetVmProcessorTool);
