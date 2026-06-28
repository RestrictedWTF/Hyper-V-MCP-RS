use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmHostInput {
    /// Hyper-V host to configure. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Maximum number of storage migrations that can be performed at the same time.
    #[serde(default, rename = "maximumStorageMigrations")]
    pub maximum_storage_migrations: Option<u32>,
    /// Maximum number of live migrations that can be performed at the same time.
    #[serde(default, rename = "maximumVirtualMachineMigrations")]
    pub maximum_virtual_machine_migrations: Option<u32>,
    /// Authentication type for live migrations: CredSSP or Kerberos.
    #[serde(default, rename = "virtualMachineMigrationAuthenticationType")]
    pub virtual_machine_migration_authentication_type: Option<String>,
    /// Whether any available network may be used for incoming live migration traffic.
    #[serde(default, rename = "useAnyNetworkForMigration")]
    pub use_any_network_for_migration: Option<bool>,
    /// Performance option for live migrations: TCPIP, Compression, or SMB.
    #[serde(default, rename = "virtualMachineMigrationPerformanceOption")]
    pub virtual_machine_migration_performance_option: Option<String>,
    /// How often resource usage data is saved, as a TimeSpan string (e.g. "01:30:00").
    #[serde(default, rename = "resourceMeteringSaveInterval")]
    pub resource_metering_save_interval: Option<String>,
    /// Default folder to store virtual hard disks on the Hyper-V host.
    #[serde(default, rename = "virtualHardDiskPath")]
    pub virtual_hard_disk_path: Option<String>,
    /// Default folder to store virtual machine configuration files on the Hyper-V host.
    #[serde(default, rename = "virtualMachinePath")]
    pub virtual_machine_path: Option<String>,
    /// Maximum dynamic MAC address, as a hexadecimal string.
    #[serde(default, rename = "macAddressMaximum")]
    pub mac_address_maximum: Option<String>,
    /// Minimum dynamic MAC address, as a hexadecimal string.
    #[serde(default, rename = "macAddressMinimum")]
    pub mac_address_minimum: Option<String>,
    /// Default World Wide Node Name for virtual Fibre Channel adapters.
    #[serde(default, rename = "fibreChannelWwnn")]
    pub fibre_channel_wwnn: Option<String>,
    /// Maximum World Wide Port Name for virtual Fibre Channel adapters.
    #[serde(default, rename = "fibreChannelWwpnMaximum")]
    pub fibre_channel_wwpn_maximum: Option<String>,
    /// Minimum World Wide Port Name for virtual Fibre Channel adapters.
    #[serde(default, rename = "fibreChannelWwpnMinimum")]
    pub fibre_channel_wwpn_minimum: Option<String>,
    /// Whether virtual machines can use resources from more than one NUMA node.
    #[serde(default, rename = "numaSpanningEnabled")]
    pub numa_spanning_enabled: Option<bool>,
    /// Whether enhanced session mode is enabled for VM connections.
    #[serde(default, rename = "enableEnhancedSessionMode")]
    pub enable_enhanced_session_mode: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostInfo {
    pub computer_name: String,
    pub fully_qualified_domain_name: String,
    pub name: String,
    pub logical_processor_count: u32,
    /// Total host memory capacity, as a string.
    pub memory_capacity: String,
    pub virtual_machine_path: String,
    pub virtual_hard_disk_path: String,
    pub enable_enhanced_session_mode: bool,
    pub numa_spanning_enabled: bool,
    pub use_any_network_for_migration: bool,
    pub virtual_machine_migration_authentication_type: String,
    pub virtual_machine_migration_performance_option: String,
    pub maximum_virtual_machine_migrations: u32,
    pub maximum_storage_migrations: u32,
    /// Resource metering save interval, as a string.
    pub resource_metering_save_interval: String,
    pub mac_address_minimum: String,
    pub mac_address_maximum: String,
    pub fibre_channel_wwnn: String,
    pub fibre_channel_wwpn_minimum: String,
    pub fibre_channel_wwpn_maximum: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmHostOutput {
    pub hosts: Vec<VmHostInfo>,
}

#[derive(Default)]
pub struct SetVmHostTool;

#[async_trait]
impl HyperVTool for SetVmHostTool {
    const NAME: &'static str = "hyperv_set_vm_host";
    const DESCRIPTION: &'static str = "Configures a Hyper-V host.";
    type Input = SetVmHostInput;
    type Output = SetVmHostOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMHost".to_string()];

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(count) = input.maximum_storage_migrations {
            args.push(format!("-MaximumStorageMigrations {}", count));
        }
        if let Some(count) = input.maximum_virtual_machine_migrations {
            args.push(format!("-MaximumVirtualMachineMigrations {}", count));
        }
        if let Some(auth) = &input.virtual_machine_migration_authentication_type {
            args.push(format!(
                "-VirtualMachineMigrationAuthenticationType '{}'",
                escape_ps_string(auth)
            ));
        }
        if let Some(use_any) = input.use_any_network_for_migration {
            args.push(format!("-UseAnyNetworkForMigration ${}", use_any));
        }
        if let Some(perf) = &input.virtual_machine_migration_performance_option {
            args.push(format!(
                "-VirtualMachineMigrationPerformanceOption '{}'",
                escape_ps_string(perf)
            ));
        }
        if let Some(interval) = &input.resource_metering_save_interval {
            args.push(format!(
                "-ResourceMeteringSaveInterval '{}'",
                escape_ps_string(interval)
            ));
        }
        if let Some(path) = &input.virtual_hard_disk_path {
            args.push(format!("-VirtualHardDiskPath '{}'", escape_ps_string(path)));
        }
        if let Some(path) = &input.virtual_machine_path {
            args.push(format!("-VirtualMachinePath '{}'", escape_ps_string(path)));
        }
        if let Some(mac) = &input.mac_address_maximum {
            args.push(format!("-MacAddressMaximum '{}'", escape_ps_string(mac)));
        }
        if let Some(mac) = &input.mac_address_minimum {
            args.push(format!("-MacAddressMinimum '{}'", escape_ps_string(mac)));
        }
        if let Some(wwnn) = &input.fibre_channel_wwnn {
            args.push(format!("-FibreChannelWwnn '{}'", escape_ps_string(wwnn)));
        }
        if let Some(wwpn) = &input.fibre_channel_wwpn_maximum {
            args.push(format!(
                "-FibreChannelWwpnMaximum '{}'",
                escape_ps_string(wwpn)
            ));
        }
        if let Some(wwpn) = &input.fibre_channel_wwpn_minimum {
            args.push(format!(
                "-FibreChannelWwpnMinimum '{}'",
                escape_ps_string(wwpn)
            ));
        }
        if let Some(enabled) = input.numa_spanning_enabled {
            args.push(format!("-NumaSpanningEnabled ${}", enabled));
        }
        if let Some(enabled) = input.enable_enhanced_session_mode {
            args.push(format!("-EnableEnhancedSessionMode ${}", enabled));
        }

        let ps = format!(
            "{} | Select-Object \
             ComputerName, FullyQualifiedDomainName, Name, LogicalProcessorCount, \
             @{{N='MemoryCapacity';E={{$_.MemoryCapacity.ToString()}}}}, \
             VirtualMachinePath, VirtualHardDiskPath, EnableEnhancedSessionMode, \
             NumaSpanningEnabled, UseAnyNetworkForMigration, \
             @{{N='VirtualMachineMigrationAuthenticationType';E={{$_.VirtualMachineMigrationAuthenticationType.ToString()}}}}, \
             @{{N='VirtualMachineMigrationPerformanceOption';E={{$_.VirtualMachineMigrationPerformanceOption.ToString()}}}}, \
             MaximumVirtualMachineMigrations, MaximumStorageMigrations, \
             @{{N='ResourceMeteringSaveInterval';E={{$_.ResourceMeteringSaveInterval.ToString()}}}}, \
             MacAddressMinimum, MacAddressMaximum, FibreChannelWwnn, \
             FibreChannelWwpnMinimum, FibreChannelWwpnMaximum | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let hosts = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(hosts.len());
        for host in hosts {
            output.push(VmHostInfo {
                computer_name: host["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                fully_qualified_domain_name: host["FullyQualifiedDomainName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                name: host["Name"].as_str().unwrap_or_default().to_string(),
                logical_processor_count: host["LogicalProcessorCount"].as_u64().unwrap_or_default()
                    as u32,
                memory_capacity: host["MemoryCapacity"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                virtual_machine_path: host["VirtualMachinePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                virtual_hard_disk_path: host["VirtualHardDiskPath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                enable_enhanced_session_mode: host["EnableEnhancedSessionMode"]
                    .as_bool()
                    .unwrap_or_default(),
                numa_spanning_enabled: host["NumaSpanningEnabled"].as_bool().unwrap_or_default(),
                use_any_network_for_migration: host["UseAnyNetworkForMigration"]
                    .as_bool()
                    .unwrap_or_default(),
                virtual_machine_migration_authentication_type: host
                    ["VirtualMachineMigrationAuthenticationType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                virtual_machine_migration_performance_option: host
                    ["VirtualMachineMigrationPerformanceOption"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                maximum_virtual_machine_migrations: host["MaximumVirtualMachineMigrations"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                maximum_storage_migrations: host["MaximumStorageMigrations"]
                    .as_u64()
                    .unwrap_or_default() as u32,
                resource_metering_save_interval: host["ResourceMeteringSaveInterval"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                mac_address_minimum: host["MacAddressMinimum"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                mac_address_maximum: host["MacAddressMaximum"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                fibre_channel_wwnn: host["FibreChannelWwnn"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                fibre_channel_wwpn_minimum: host["FibreChannelWwpnMinimum"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                fibre_channel_wwpn_maximum: host["FibreChannelWwpnMaximum"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmHostOutput { hosts: output })
    }
}

register_tool!(SetVmHostTool);
