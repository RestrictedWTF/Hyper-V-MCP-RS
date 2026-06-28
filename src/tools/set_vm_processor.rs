use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmProcessorInput {
    /// Name of the virtual machine whose processors are to be configured.
pub name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Number of virtual processors for the virtual machine.
    #[serde(default)]
    pub count: Option<i64>,
    /// Enables processor compatibility for live migration.
    #[serde(default, rename = "compatibilityForMigrationEnabled")]
    pub compatibility_for_migration_enabled: Option<bool>,
    /// Enables processor compatibility for older operating systems.
    #[serde(default, rename = "compatibilityForOlderOperatingSystemsEnabled")]
    pub compatibility_for_older_operating_systems_enabled: Option<bool>,
    /// Number of virtual SMT threads exposed per core. 0 means inherit from the host.
    #[serde(default, rename = "hwThreadCountPerCore")]
    pub hw_thread_count_per_core: Option<i64>,
    /// Maximum percentage of host processor resources available to the VM (0-100).
    #[serde(default)]
    pub maximum: Option<i64>,
    /// Percentage of processor resources reserved for the VM (0-100).
    #[serde(default)]
    pub reserve: Option<i64>,
    /// Relative weight for CPU allocation (1-10000).
    #[serde(default, rename = "relativeWeight")]
    pub relative_weight: Option<i32>,
    /// Maximum number of processors per NUMA node.
    #[serde(default, rename = "maximumCountPerNumaNode")]
    pub maximum_count_per_numa_node: Option<i32>,
    /// Maximum number of sockets per NUMA node.
    #[serde(default, rename = "maximumCountPerNumaSocket")]
    pub maximum_count_per_numa_socket: Option<i32>,
    /// Name of the processor resource pool to be used.
    #[serde(default, rename = "resourcePoolName")]
    pub resource_pool_name: Option<String>,
    /// Performance monitoring hardware to expose (e.g. "pmu,pebs,lbr").
    #[serde(default)]
    pub perfmon: Option<String>,
    /// Enables host resource protection for the virtual machine.
    #[serde(default, rename = "enableHostResourceProtection")]
    pub enable_host_resource_protection: Option<bool>,
    /// Exposes virtualization extensions to the VM to enable nested virtualization.
    #[serde(default, rename = "exposeVirtualizationExtensions")]
    pub expose_virtualization_extensions: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmProcessorInfo {
    pub vm_name: String,
    pub count: i64,
    pub compatibility_for_migration_enabled: bool,
    pub compatibility_for_older_operating_systems_enabled: bool,
    pub hw_thread_count_per_core: i64,
    pub maximum: i64,
    pub reserve: i64,
    pub relative_weight: i32,
    pub maximum_count_per_numa_node: i32,
    pub maximum_count_per_numa_socket: i32,
    pub resource_pool_name: String,
    pub perfmon: String,
    pub enable_host_resource_protection: bool,
    pub expose_virtualization_extensions: bool,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmProcessorOutput {
    pub processors: Vec<VmProcessorInfo>,
}

#[derive(Default)]
pub struct SetVmProcessorTool;

#[async_trait]
impl HyperVTool for SetVmProcessorTool {
    const NAME: &'static str = "hyperv_set_vm_processor";
    const DESCRIPTION: &'static str =
        "Configures settings for the virtual processors of a virtual machine. Settings are applied uniformly to all virtual processors belonging to the virtual machine.";
    type Input = SetVmProcessorInput;
    type Output = SetVmProcessorOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMProcessor -VMName '{}'",
            escape_ps_string(&input.name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(count) = input.count {
            args.push(format!("-Count {}", count));
        }
        if let Some(enabled) = input.compatibility_for_migration_enabled {
            args.push(format!("-CompatibilityForMigrationEnabled ${}", enabled));
        }
        if let Some(enabled) = input.compatibility_for_older_operating_systems_enabled {
            args.push(format!(
                "-CompatibilityForOlderOperatingSystemsEnabled ${}",
                enabled
            ));
        }
        if let Some(threads) = input.hw_thread_count_per_core {
            args.push(format!("-HwThreadCountPerCore {}", threads));
        }
        if let Some(maximum) = input.maximum {
            args.push(format!("-Maximum {}", maximum));
        }
        if let Some(reserve) = input.reserve {
            args.push(format!("-Reserve {}", reserve));
        }
        if let Some(weight) = input.relative_weight {
            args.push(format!("-RelativeWeight {}", weight));
        }
        if let Some(count) = input.maximum_count_per_numa_node {
            args.push(format!("-MaximumCountPerNumaNode {}", count));
        }
        if let Some(count) = input.maximum_count_per_numa_socket {
            args.push(format!("-MaximumCountPerNumaSocket {}", count));
        }
        if let Some(pool) = &input.resource_pool_name {
            args.push(format!("-ResourcePoolName '{}'", escape_ps_string(pool)));
        }
        if let Some(perfmon) = &input.perfmon {
            args.push(format!("-Perfmon '{}'", escape_ps_string(perfmon)));
        }
        if let Some(enabled) = input.enable_host_resource_protection {
            args.push(format!("-EnableHostResourceProtection ${}", enabled));
        }
        if let Some(enabled) = input.expose_virtualization_extensions {
            args.push(format!("-ExposeVirtualizationExtensions ${}", enabled));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             VMName, Count, CompatibilityForMigrationEnabled, \
             CompatibilityForOlderOperatingSystemsEnabled, HwThreadCountPerCore, \
             Maximum, Reserve, RelativeWeight, MaximumCountPerNumaNode, \
             MaximumCountPerNumaSocket, ResourcePoolName, Perfmon, \
             EnableHostResourceProtection, ExposeVirtualizationExtensions, \
             ComputerName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

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
            output.push(VmProcessorInfo {
                vm_name: proc["VMName"].as_str().unwrap_or_default().to_string(),
                count: proc["Count"].as_i64().unwrap_or_default(),
                compatibility_for_migration_enabled: proc["CompatibilityForMigrationEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
                compatibility_for_older_operating_systems_enabled: proc
                    ["CompatibilityForOlderOperatingSystemsEnabled"]
                    .as_bool()
                    .unwrap_or_default(),
                hw_thread_count_per_core: proc["HwThreadCountPerCore"].as_i64().unwrap_or_default(),
                maximum: proc["Maximum"].as_i64().unwrap_or_default(),
                reserve: proc["Reserve"].as_i64().unwrap_or_default(),
                relative_weight: proc["RelativeWeight"].as_i64().unwrap_or_default() as i32,
                maximum_count_per_numa_node: proc["MaximumCountPerNumaNode"]
                    .as_i64()
                    .unwrap_or_default() as i32,
                maximum_count_per_numa_socket: proc["MaximumCountPerNumaSocket"]
                    .as_i64()
                    .unwrap_or_default() as i32,
                resource_pool_name: proc["ResourcePoolName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                perfmon: proc["Perfmon"].as_str().unwrap_or_default().to_string(),
                enable_host_resource_protection: proc["EnableHostResourceProtection"]
                    .as_bool()
                    .unwrap_or_default(),
                expose_virtualization_extensions: proc["ExposeVirtualizationExtensions"]
                    .as_bool()
                    .unwrap_or_default(),
                computer_name: proc["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmProcessorOutput { processors: output })
    }
}

register_tool!(SetVmProcessorTool);
