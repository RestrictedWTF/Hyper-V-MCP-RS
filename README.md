# Hyper-V MCP

A production-ready [Model Context Protocol](https://modelcontextprotocol.io/) server for managing Microsoft Hyper-V, built with the official [`modelcontextprotocol/rust-sdk`](https://github.com/modelcontextprotocol/rust-sdk) (`rmcp`).

Exposes the full [Hyper-V PowerShell module](https://learn.microsoft.com/en-us/powershell/module/hyper-v/) as typed MCP tools and read-only resources, accessible from any MCP-compatible host (Claude Desktop, Cline, etc.).

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  MCP Host (Claude Desktop / Cline / etc.)                   │
│  Spawns: hyperv-mcp.exe via stdio                           │
└──────────────┬──────────────────────────────────────────────┘
               │ JSON-RPC over stdin/stdout
┌──────────────▼──────────────────────────────────────────────┐
│  Rust binary: hyperv-mcp                                    │
│  • tokio async runtime                                      │
│  • rmcp ServerHandler (manual impl)                         │
│  • Hyper-V tool registry (collected at link time)           │
│  • PowerShell sidecar manager (spawn, restart, timeout)     │
└──────────────┬──────────────────────────────────────────────┘
               │ JSON-RPC over child stdin/stdout
┌──────────────▼──────────────────────────────────────────────┐
│  PowerShell sidecar (embedded via include_str!)             │
│  • Elevation check on init                                  │
│  • Persistent runspace loop                                 │
│  • Execute Hyper-V cmdlets, return JSON                     │
└─────────────────────────────────────────────────────────────┘
```

**Transport:** `stdio` only. The binary communicates with MCP hosts over `stdin`/`stdout`. The PowerShell sidecar is spawned as a persistent child process, embedded via `include_str!` — no temp files on disk.

**Sidecar lifecycle:** On first call the sidecar performs an elevation check and emits `{"ready":true}`. If the sidecar crashes, hangs, or exceeds the per-call timeout, the Rust layer kills and restarts it automatically. Pending calls fail with a structured `ToolError::Sidecar`.

---

## Requirements

- Windows with Hyper-V enabled
- Administrative privileges (the server will exit with code 1 if not elevated)
- PowerShell 5.1+ (inbox on Windows 10/Server 2016+)
- Rust toolchain (stable)

---

## Building

```sh
cargo build --release
```

The resulting binary is `target/release/hyperv-mcp.exe`.

---

## MCP Host Configuration

Add the server to your MCP host config. Example for Claude Desktop (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "hyper-v": {
      "command": "C:\\path\\to\\hyperv-mcp.exe"
    }
  }
}
```

> The process must be launched with Administrator privileges. On Claude Desktop, ensure the host itself is running elevated, or use a launcher wrapper.

---

## Resources

Read-only inventory and topology data, polled on demand:

| URI | Description |
|---|---|
| `hyperv://vms/inventory` | All VMs on the host — name, ID, state |
| `hyperv://networks/topology` | Virtual switches and their adapters |
| `hyperv://host/info` | Host name, version, supported VM config versions |

---

## Guest Control

PowerShell Direct tools allow the agent to execute commands and transfer files inside a running VM over VMBus — no network connectivity required.

### Credential Resolution

Guest tools resolve credentials in this order:

1. Explicit `username` / `password` supplied in the tool call
2. VM-specific entry in the DPAPI-encrypted credential store (`%APPDATA%\hyperv-mcp\credentials.json`)
3. Default credential from the MCP config file (`%APPDATA%\hyperv-mcp\config.json`)
4. Error — call `hyperv_register_vm_credential` to register credentials for the VM

### Config File

`%APPDATA%\hyperv-mcp\config.json` is read at startup and holds the default guest credential:

```json
{
  "default_credential": {
    "username": "Administrator",
    "password": "<DPAPI-encrypted blob>"
  }
}
```

This is intended for use with a known base VHDX whose local admin password is fixed. All agent-created VMs are assumed to derive from this image.

### Credential Store

Per-VM credentials are stored in `%APPDATA%\hyperv-mcp\credentials.json`, DPAPI-encrypted at rest. Entries are keyed by VM name. Use `hyperv_register_vm_credential` to add or update entries.

### Session Model

Each guest tool call creates its own transient PowerShell Direct session, performs the operation, and tears it down. No session IDs are exposed to the agent.

### Guest Tools

| Tool | Description |
|---|---|
| `hyperv_invoke_guest_command` | Runs a PowerShell script block inside the guest via `Invoke-Command -VMName`. Returns stdout, stderr, and exit code as JSON. |
| `hyperv_copy_to_guest` | Copies a file from the host into the guest using a transient `PSSession`. |
| `hyperv_copy_from_guest` | Copies a file from the guest to the host using a transient `PSSession`. |
| `hyperv_register_vm_credential` | Stores a username/password for a specific VM in the DPAPI-encrypted credential store. |

---

## Tools

Every Hyper-V cmdlet is exposed as a typed MCP tool. Tool names follow the pattern `hyperv_<verb>_<noun>` (e.g. `hyperv_get_vm`, `hyperv_start_vm`, `hyperv_new_vhd`).

Input and output schemas are derived from Rust structs via `schemars` and surfaced to the MCP host as JSON Schema.

### Add

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_add_vm_assignable_device` | `Add-VMAssignableDevice` | Adds an assignable device to a specific virtual machine. |
| `hyperv_add_vm_dvd_drive` | `Add-VMDvdDrive` | Adds a DVD drive to a virtual machine. |
| `hyperv_add_vm_fibre_channel_hba` | `Add-VMFibreChannelHba` | Adds a virtual Fibre Channel host bus adapter to a virtual machine. |
| `hyperv_add_vm_gpu_partition_adapter` | `Add-VMGpuPartitionAdapter` | Adds a GPU partition adapter to a virtual machine. |
| `hyperv_add_vm_group_member` | `Add-VMGroupMember` | Adds group members to a virtual machine group. |
| `hyperv_add_vm_hard_disk_drive` | `Add-VMHardDiskDrive` | Adds a hard disk drive to a virtual machine. |
| `hyperv_add_vm_host_assignable_device` | `Add-VMHostAssignableDevice` | Adds an assignable device to a VM host. |
| `hyperv_add_vm_migration_network` | `Add-VMMigrationNetwork` | Adds a network for virtual machine migration on one or more hosts. |
| `hyperv_add_vm_network_adapter` | `Add-VMNetworkAdapter` | Adds a virtual network adapter to a virtual machine. |
| `hyperv_add_vm_network_adapter_acl` | `Add-VMNetworkAdapterAcl` | Creates an ACL to apply to traffic through a virtual machine network adapter. |
| `hyperv_add_vm_network_adapter_extended_acl` | `Add-VMNetworkAdapterExtendedAcl` | Creates an extended ACL for a virtual network adapter. |
| `hyperv_add_vm_network_adapter_routing_domain_mapping` | `Add-VMNetworkAdapterRoutingDomainMapping` | Adds a routing domain and virtual subnets to a virtual network adapter. |
| `hyperv_add_vm_remote_fx_3d_video_adapter` | `Add-VMRemoteFx3dVideoAdapter` | Adds a RemoteFX video adapter in a virtual machine. |
| `hyperv_add_vm_scsi_controller` | `Add-VMScsiController` | Adds a SCSI controller in a virtual machine. |
| `hyperv_add_vm_storage_path` | `Add-VMStoragePath` | Adds a path to a storage resource pool. |
| `hyperv_add_vm_switch` | `Add-VMSwitch` | Adds a virtual switch to an Ethernet resource pool. |
| `hyperv_add_vm_switch_extension_port_feature` | `Add-VMSwitchExtensionPortFeature` | Adds a feature to a virtual network adapter. |
| `hyperv_add_vm_switch_extension_switch_feature` | `Add-VMSwitchExtensionSwitchFeature` | Adds a feature to a virtual switch. |
| `hyperv_add_vm_switch_team_member` | `Add-VMSwitchTeamMember` | Adds members to a virtual switch team. |

### Checkpoint / Compare / Complete

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_checkpoint_vm` | `Checkpoint-VM` | Creates a checkpoint of a virtual machine. |
| `hyperv_compare_vm` | `Compare-VM` | Compares a VM and a host for compatibility, returning a compatibility report. |
| `hyperv_complete_vm_failover` | `Complete-VMFailover` | Completes a VM's failover process on the Replica server. |

### Connect / Convert / Copy

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_connect_vm_network_adapter` | `Connect-VMNetworkAdapter` | Connects a virtual network adapter to a virtual switch. |
| `hyperv_connect_vm_san` | `Connect-VMSan` | Associates a host bus adapter with a virtual SAN. |
| `hyperv_convert_vhd` | `Convert-VHD` | Converts the format, version type, and block size of a VHD file. |
| `hyperv_copy_vm_file` | `Copy-VMFile` | Copies a file to a virtual machine. |

### Debug / Disable

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_debug_vm` | `Debug-VM` | Debugs a virtual machine. |
| `hyperv_disable_vm_console_support` | `Disable-VMConsoleSupport` | Disables keyboard, video, and mouse for a generation 2 virtual machine. |
| `hyperv_disable_vm_eventing` | `Disable-VMEventing` | Disables virtual machine eventing. |
| `hyperv_disable_vm_integration_service` | `Disable-VMIntegrationService` | Disables an integration service on a virtual machine. |
| `hyperv_disable_vm_migration` | `Disable-VMMigration` | Disables migration on one or more virtual machine hosts. |
| `hyperv_disable_vm_remote_fx_physical_video_adapter` | `Disable-VMRemoteFXPhysicalVideoAdapter` | Disables one or more RemoteFX physical video adapters. |
| `hyperv_disable_vm_resource_metering` | `Disable-VMResourceMetering` | Disables collection of resource utilization data for a VM or resource pool. |
| `hyperv_disable_vm_switch_extension` | `Disable-VMSwitchExtension` | Disables one or more extensions on one or more virtual switches. |
| `hyperv_disable_vm_tpm` | `Disable-VMTPM` | Disables TPM functionality on a virtual machine. |

### Disconnect / Dismount

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_disconnect_vm_network_adapter` | `Disconnect-VMNetworkAdapter` | Disconnects a virtual network adapter from a virtual switch or Ethernet resource pool. |
| `hyperv_disconnect_vm_san` | `Disconnect-VMSan` | Removes a host bus adapter from a virtual SAN. |
| `hyperv_dismount_vhd` | `Dismount-VHD` | Dismounts a virtual hard disk. |
| `hyperv_dismount_vm_host_assignable_device` | `Dismount-VMHostAssignableDevice` | Dismounts a device from a VM host. |

### Enable

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_enable_vm_console_support` | `Enable-VMConsoleSupport` | Enables keyboard, video, and mouse for virtual machines. |
| `hyperv_enable_vm_eventing` | `Enable-VMEventing` | Enables virtual machine eventing. |
| `hyperv_enable_vm_integration_service` | `Enable-VMIntegrationService` | Enables an integration service on a virtual machine. |
| `hyperv_enable_vm_migration` | `Enable-VMMigration` | Enables migration on one or more virtual machine hosts. |
| `hyperv_enable_vm_remote_fx_physical_video_adapter` | `Enable-VMRemoteFXPhysicalVideoAdapter` | Enables one or more RemoteFX physical video adapters. |
| `hyperv_enable_vm_replication` | `Enable-VMReplication` | Enables replication of a virtual machine. |
| `hyperv_enable_vm_resource_metering` | `Enable-VMResourceMetering` | Collects resource utilization data for a VM or resource pool. |
| `hyperv_enable_vm_switch_extension` | `Enable-VMSwitchExtension` | Enables one or more extensions on one or more switches. |
| `hyperv_enable_vm_tpm` | `Enable-VMTPM` | Enables TPM functionality on a virtual machine. |

### Export

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_export_vm` | `Export-VM` | Exports a virtual machine to disk. |
| `hyperv_export_vm_snapshot` | `Export-VMSnapshot` | Exports a virtual machine checkpoint to disk. |

### Get

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_get_vhd` | `Get-VHD` | Gets the virtual hard disk object associated with a VHD. |
| `hyperv_get_vhd_set` | `Get-VHDSet` | Gets information about a VHD set. |
| `hyperv_get_vhd_snapshot` | `Get-VHDSnapshot` | Gets information about a checkpoint in a VHD set. |
| `hyperv_get_vm` | `Get-VM` | Gets virtual machines from one or more Hyper-V hosts. |
| `hyperv_get_vm_assignable_device` | `Get-VMAssignableDevice` | Retrieves information about the assignable device from a specific VM. |
| `hyperv_get_vm_bios` | `Get-VMBios` | Gets the BIOS of a virtual machine or snapshot. |
| `hyperv_get_vm_com_port` | `Get-VMComPort` | Gets the COM ports of a virtual machine or snapshot. |
| `hyperv_get_vm_connect_access` | `Get-VMConnectAccess` | Gets entries showing users and the VMs they can connect to on one or more hosts. |
| `hyperv_get_vm_dvd_drive` | `Get-VMDvdDrive` | Gets the DVD drives attached to a virtual machine or snapshot. |
| `hyperv_get_vm_fibre_channel_hba` | `Get-VMFibreChannelHba` | Gets the Fibre Channel HBAs associated with one or more virtual machines. |
| `hyperv_get_vm_firmware` | `Get-VMFirmware` | Gets the firmware configuration of a virtual machine. |
| `hyperv_get_vm_floppy_disk_drive` | `Get-VMFloppyDiskDrive` | Gets the floppy disk drives of a virtual machine or snapshot. |
| `hyperv_get_vm_gpu_partition_adapter` | `Get-VMGpuPartitionAdapter` | Gets the information of assigned GPU partitions to a virtual machine. |
| `hyperv_get_vm_group` | `Get-VMGroup` | Gets virtual machine groups. |
| `hyperv_get_vm_hard_disk_drive` | `Get-VMHardDiskDrive` | Gets the virtual hard disk drives attached to one or more virtual machines. |
| `hyperv_get_vm_host` | `Get-VMHost` | Gets a Hyper-V host. |
| `hyperv_get_vm_host_assignable_device` | `Get-VMHostAssignableDevice` | Retrieves device information assigned to a VM host. |
| `hyperv_get_vm_host_cluster` | `Get-VMHostCluster` | Gets virtual machine host clusters. |
| `hyperv_get_vm_host_numa_node` | `Get-VMHostNumaNode` | Gets the NUMA topology of a virtual machine host. |
| `hyperv_get_vm_host_numa_node_status` | `Get-VMHostNumaNodeStatus` | Gets the status of VMs on the NUMA nodes of a host. |
| `hyperv_get_vm_host_partitionable_gpu` | `Get-VMHostPartitionableGpu` | Gets the host machine's partitionable GPU. |
| `hyperv_get_vm_host_supported_version` | `Get-VMHostSupportedVersion` | Returns a list of VM configuration versions supported on a host. |
| `hyperv_get_vm_ide_controller` | `Get-VMIdeController` | Gets the IDE controllers of a virtual machine or snapshot. |
| `hyperv_get_vm_integration_service` | `Get-VMIntegrationService` | Gets the integration services of a virtual machine or snapshot. |
| `hyperv_get_vm_key_protector` | `Get-VMKeyProtector` | Retrieves a key protector for a virtual machine. |
| `hyperv_get_vm_memory` | `Get-VMMemory` | Gets the memory of a virtual machine or snapshot. |
| `hyperv_get_vm_migration_network` | `Get-VMMigrationNetwork` | Gets the networks added for migration to one or more VM hosts. |
| `hyperv_get_vm_network_adapter` | `Get-VMNetworkAdapter` | Gets the virtual network adapters of a VM, snapshot, or management OS. |
| `hyperv_get_vm_network_adapter_acl` | `Get-VMNetworkAdapterAcl` | Gets the ACLs configured for a virtual machine network adapter. |
| `hyperv_get_vm_network_adapter_extended_acl` | `Get-VMNetworkAdapterExtendedAcl` | Gets extended ACLs configured for a virtual network adapter. |
| `hyperv_get_vm_network_adapter_failover_configuration` | `Get-VMNetworkAdapterFailoverConfiguration` | Gets the IP address of a virtual network adapter configured for failover. |
| `hyperv_get_vm_network_adapter_isolation` | `Get-VMNetworkAdapterIsolation` | Gets isolation settings for a virtual network adapter. |
| `hyperv_get_vm_network_adapter_routing_domain_mapping` | `Get-VMNetworkAdapterRoutingDomainMapping` | Gets members of a routing domain. |
| `hyperv_get_vm_network_adapter_team_mapping` | `Get-VMNetworkAdapterTeamMapping` | Gets the team mapping settings configured on a virtual network adapter. |
| `hyperv_get_vm_network_adapter_vlan` | `Get-VMNetworkAdapterVlan` | Gets the virtual LAN settings configured on a virtual network adapter. |
| `hyperv_get_vm_processor` | `Get-VMProcessor` | Gets the processor of a virtual machine or snapshot. |
| `hyperv_get_vm_remote_fx_3d_video_adapter` | `Get-VMRemoteFx3dVideoAdapter` | Gets the RemoteFX video adapter of a virtual machine or snapshot. |
| `hyperv_get_vm_remote_fx_physical_video_adapter` | `Get-VMRemoteFXPhysicalVideoAdapter` | Gets the RemoteFX physical graphics adapters on one or more hosts. |
| `hyperv_get_vm_replication` | `Get-VMReplication` | Gets the replication settings for a virtual machine. |
| `hyperv_get_vm_replication_authorization_entry` | `Get-VMReplicationAuthorizationEntry` | Gets the authorization entries of a Replica server. |
| `hyperv_get_vm_replication_server` | `Get-VMReplicationServer` | Gets the replication and authentication settings of a Replica server. |
| `hyperv_get_vm_resource_pool` | `Get-VMResourcePool` | Gets the resource pools on one or more virtual machine hosts. |
| `hyperv_get_vm_san` | `Get-VMSan` | Gets the available VM storage area networks on a Hyper-V host. |
| `hyperv_get_vm_scsi_controller` | `Get-VMScsiController` | Gets the SCSI controllers of a virtual machine or snapshot. |
| `hyperv_get_vm_security` | `Get-VMSecurity` | Gets security information about a virtual machine. |
| `hyperv_get_vm_snapshot` | `Get-VMSnapshot` | Gets the checkpoints associated with a virtual machine or checkpoint. |
| `hyperv_get_vm_storage_path` | `Get-VMStoragePath` | Gets the storage paths in a storage resource pool. |
| `hyperv_get_vm_switch` | `Get-VMSwitch` | Gets virtual switches from one or more virtual Hyper-V hosts. |
| `hyperv_get_vm_switch_extension` | `Get-VMSwitchExtension` | Gets the extensions on one or more virtual switches. |
| `hyperv_get_vm_switch_extension_port_data` | `Get-VMSwitchExtensionPortData` | Retrieves the status of a virtual switch extension feature applied to a virtual network adapter. |
| `hyperv_get_vm_switch_extension_port_feature` | `Get-VMSwitchExtensionPortFeature` | Gets the features configured on a virtual network adapter. |
| `hyperv_get_vm_switch_extension_switch_data` | `Get-VMSwitchExtensionSwitchData` | Gets the status of a virtual switch extension feature applied on a virtual switch. |
| `hyperv_get_vm_switch_extension_switch_feature` | `Get-VMSwitchExtensionSwitchFeature` | Gets the features configured on a virtual switch. |
| `hyperv_get_vm_switch_team` | `Get-VMSwitchTeam` | Gets virtual switch teams from Hyper-V hosts. |
| `hyperv_get_vm_system_switch_extension` | `Get-VMSystemSwitchExtension` | Gets the switch extensions installed on a virtual machine host. |
| `hyperv_get_vm_system_switch_extension_port_feature` | `Get-VMSystemSwitchExtensionPortFeature` | Gets the port-level features supported by virtual switch extensions on one or more hosts. |
| `hyperv_get_vm_system_switch_extension_switch_feature` | `Get-VMSystemSwitchExtensionSwitchFeature` | Gets the switch-level features on one or more Hyper-V hosts. |
| `hyperv_get_vm_video` | `Get-VMVideo` | Gets video settings for virtual machines. |

### Grant / Import

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_grant_vm_connect_access` | `Grant-VMConnectAccess` | Grants a user or users access to connect to a virtual machine. |
| `hyperv_import_vm` | `Import-VM` | Imports a virtual machine from a file. |
| `hyperv_import_vm_initial_replication` | `Import-VMInitialReplication` | Imports initial replication files for a Replica VM to complete initial replication from external media. |

### Measure / Merge / Mount / Move

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_measure_vm` | `Measure-VM` | Reports resource utilization data for one or more virtual machines. |
| `hyperv_measure_vm_replication` | `Measure-VMReplication` | Gets replication statistics and information associated with a virtual machine. |
| `hyperv_measure_vm_resource_pool` | `Measure-VMResourcePool` | Reports resource utilization data for one or more resource pools. |
| `hyperv_merge_vhd` | `Merge-VHD` | Merges virtual hard disks. |
| `hyperv_mount_vhd` | `Mount-VHD` | Mounts one or more virtual hard disks. |
| `hyperv_mount_vm_host_assignable_device` | `Mount-VMHostAssignableDevice` | Mounts a device to a VM host. |
| `hyperv_move_vm` | `Move-VM` | Moves a virtual machine to a new Hyper-V host. |
| `hyperv_move_vm_storage` | `Move-VMStorage` | Moves the storage of a virtual machine. |

### New

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_new_vfd` | `New-VFD` | Creates a virtual floppy disk. |
| `hyperv_new_vhd` | `New-VHD` | Creates one or more new virtual hard disks. |
| `hyperv_new_vm` | `New-VM` | Creates a new virtual machine. |
| `hyperv_new_vm_group` | `New-VMGroup` | Creates a virtual machine group. |
| `hyperv_new_vm_replication_authorization_entry` | `New-VMReplicationAuthorizationEntry` | Creates a new authorization entry allowing primary servers to replicate to a Replica server. |
| `hyperv_new_vm_resource_pool` | `New-VMResourcePool` | Creates a resource pool. |
| `hyperv_new_vm_san` | `New-VMSan` | Creates a new virtual SAN on a Hyper-V host. |
| `hyperv_new_vm_switch` | `New-VMSwitch` | Creates a new virtual switch on one or more VM hosts. |

### Optimize

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_optimize_vhd` | `Optimize-VHD` | Optimizes the allocation of space used by VHD files (not fixed disks). |
| `hyperv_optimize_vhd_set` | `Optimize-VHDSet` | Optimizes VHD set files. |

### Remove

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_remove_vhd_snapshot` | `Remove-VHDSnapshot` | Removes a checkpoint from a VHD set file. |
| `hyperv_remove_vm` | `Remove-VM` | Deletes a virtual machine. |
| `hyperv_remove_vm_assignable_device` | `Remove-VMAssignableDevice` | Removes information about the assignable devices from a specific VM. |
| `hyperv_remove_vm_dvd_drive` | `Remove-VMDvdDrive` | Deletes a DVD drive from a virtual machine. |
| `hyperv_remove_vm_fibre_channel_hba` | `Remove-VMFibreChannelHba` | Removes a Fibre Channel HBA from a virtual machine. |
| `hyperv_remove_vm_gpu_partition_adapter` | `Remove-VMGpuPartitionAdapter` | Removes an assigned GPU partition from a virtual machine. |
| `hyperv_remove_vm_group` | `Remove-VMGroup` | Removes a virtual machine group. |
| `hyperv_remove_vm_group_member` | `Remove-VMGroupMember` | Removes members from a virtual machine group. |
| `hyperv_remove_vm_hard_disk_drive` | `Remove-VMHardDiskDrive` | Deletes a hard disk drive from a virtual machine. |
| `hyperv_remove_vm_host_assignable_device` | `Remove-VMHostAssignableDevice` | Removes a device assigned to a VM host. |
| `hyperv_remove_vm_migration_network` | `Remove-VMMigrationNetwork` | Removes a network from use with migration. |
| `hyperv_remove_vm_network_adapter` | `Remove-VMNetworkAdapter` | Removes one or more virtual network adapters from a virtual machine. |
| `hyperv_remove_vm_network_adapter_acl` | `Remove-VMNetworkAdapterAcl` | Removes an ACL applied to traffic through a virtual network adapter. |
| `hyperv_remove_vm_network_adapter_extended_acl` | `Remove-VMNetworkAdapterExtendedAcl` | Removes an extended ACL for a virtual network adapter. |
| `hyperv_remove_vm_network_adapter_routing_domain_mapping` | `Remove-VMNetworkAdapterRoutingDomainMapping` | Removes a routing domain from a virtual network adapter. |
| `hyperv_remove_vm_network_adapter_team_mapping` | `Remove-VMNetworkAdapterTeamMapping` | Removes the team mapping settings from a virtual network adapter. |
| `hyperv_remove_vm_remote_fx_3d_video_adapter` | `Remove-VMRemoteFx3dVideoAdapter` | Removes a RemoteFX 3D video adapter from a virtual machine. |
| `hyperv_remove_vm_replication` | `Remove-VMReplication` | Removes the replication relationship of a virtual machine. |
| `hyperv_remove_vm_replication_authorization_entry` | `Remove-VMReplicationAuthorizationEntry` | Removes an authorization entry from a Replica server. |
| `hyperv_remove_vm_resource_pool` | `Remove-VMResourcePool` | Deletes a resource pool from one or more VM hosts. |
| `hyperv_remove_vm_san` | `Remove-VMSan` | Removes a virtual SAN from a Hyper-V host. |
| `hyperv_remove_vm_saved_state` | `Remove-VMSavedState` | Deletes the saved state of a saved virtual machine. |
| `hyperv_remove_vm_scsi_controller` | `Remove-VMScsiController` | Removes a SCSI controller from a virtual machine. |
| `hyperv_remove_vm_snapshot` | `Remove-VMSnapshot` | Deletes a virtual machine checkpoint. |
| `hyperv_remove_vm_storage_path` | `Remove-VMStoragePath` | Removes a path from a storage resource pool. |
| `hyperv_remove_vm_switch` | `Remove-VMSwitch` | Deletes a virtual switch. |
| `hyperv_remove_vm_switch_extension_port_feature` | `Remove-VMSwitchExtensionPortFeature` | Removes a feature from a virtual network adapter. |
| `hyperv_remove_vm_switch_extension_switch_feature` | `Remove-VMSwitchExtensionSwitchFeature` | Removes a feature from a virtual switch. |
| `hyperv_remove_vm_switch_team_member` | `Remove-VMSwitchTeamMember` | Removes a member from a virtual machine switch team. |

### Rename

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_rename_vm` | `Rename-VM` | Renames a virtual machine. |
| `hyperv_rename_vm_group` | `Rename-VMGroup` | Renames virtual machine groups. |
| `hyperv_rename_vm_network_adapter` | `Rename-VMNetworkAdapter` | Renames a virtual network adapter on a VM or management OS. |
| `hyperv_rename_vm_resource_pool` | `Rename-VMResourcePool` | Renames a resource pool on one or more Hyper-V hosts. |
| `hyperv_rename_vm_san` | `Rename-VMSan` | Renames a virtual storage area network. |
| `hyperv_rename_vm_snapshot` | `Rename-VMSnapshot` | Renames a virtual machine checkpoint. |
| `hyperv_rename_vm_switch` | `Rename-VMSwitch` | Renames a virtual switch. |

### Repair / Reset / Resize / Restart / Restore / Resume / Revoke

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_repair_vm` | `Repair-VM` | Repairs one or more virtual machines. |
| `hyperv_reset_vm_replication_statistics` | `Reset-VMReplicationStatistics` | Resets the replication statistics of a virtual machine. |
| `hyperv_reset_vm_resource_metering` | `Reset-VMResourceMetering` | Resets the resource utilization data collected by Hyper-V resource metering. |
| `hyperv_resize_vhd` | `Resize-VHD` | Resizes a virtual hard disk. |
| `hyperv_restart_vm` | `Restart-VM` | Restarts a virtual machine. |
| `hyperv_restore_vm_snapshot` | `Restore-VMSnapshot` | Restores a virtual machine checkpoint. |
| `hyperv_resume_vm` | `Resume-VM` | Resumes a suspended (paused) virtual machine. |
| `hyperv_resume_vm_replication` | `Resume-VMReplication` | Resumes a VM replication in a Paused, Error, Resynchronization Required, or Suspended state. |
| `hyperv_revoke_vm_connect_access` | `Revoke-VMConnectAccess` | Revokes access for one or more users to connect to one or more virtual machines. |

### Save / Set

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_save_vm` | `Save-VM` | Saves a virtual machine. |
| `hyperv_set_vhd` | `Set-VHD` | Sets properties associated with a virtual hard disk. |
| `hyperv_set_vm` | `Set-VM` | Configures a virtual machine. |
| `hyperv_set_vm_bios` | `Set-VMBios` | Configures the BIOS of a Generation 1 virtual machine. |
| `hyperv_set_vm_com_port` | `Set-VMComPort` | Configures the COM port of a virtual machine. |
| `hyperv_set_vm_dvd_drive` | `Set-VMDvdDrive` | Configures a virtual DVD drive. |
| `hyperv_set_vm_fibre_channel_hba` | `Set-VMFibreChannelHba` | Configures a Fibre Channel HBA on a virtual machine. |
| `hyperv_set_vm_firmware` | `Set-VMFirmware` | Sets the firmware configuration of a virtual machine. |
| `hyperv_set_vm_floppy_disk_drive` | `Set-VMFloppyDiskDrive` | Configures a virtual floppy disk drive. |
| `hyperv_set_vm_gpu_partition_adapter` | `Set-VMGpuPartitionAdapter` | Assigns a partition of a GPU to a virtual machine. |
| `hyperv_set_vm_hard_disk_drive` | `Set-VMHardDiskDrive` | Configures a virtual hard disk. |
| `hyperv_set_vm_host` | `Set-VMHost` | Configures a Hyper-V host. |
| `hyperv_set_vm_host_cluster` | `Set-VMHostCluster` | Configures a virtual machine host cluster. |
| `hyperv_set_vm_host_partitionable_gpu` | `Set-VMHostPartitionableGpu` | Configures the host partitionable GPU to the number of partitions supported by the manufacturer. |
| `hyperv_set_vm_key_protector` | `Set-VMKeyProtector` | Configures a key protector for a virtual machine. |
| `hyperv_set_vm_memory` | `Set-VMMemory` | Configures the memory of a virtual machine. |
| `hyperv_set_vm_migration_network` | `Set-VMMigrationNetwork` | Sets the subnet, subnet mask, and/or priority of a migration network. |
| `hyperv_set_vm_network_adapter` | `Set-VMNetworkAdapter` | Configures features of the virtual network adapter in a VM or management OS. |
| `hyperv_set_vm_network_adapter_failover_configuration` | `Set-VMNetworkAdapterFailoverConfiguration` | Configures the IP address of a virtual network adapter to be used on failover. |
| `hyperv_set_vm_network_adapter_isolation` | `Set-VMNetworkAdapterIsolation` | Modifies isolation settings for a virtual network adapter. |
| `hyperv_set_vm_network_adapter_routing_domain_mapping` | `Set-VMNetworkAdapterRoutingDomainMapping` | Sets virtual subnets on a routing domain. |
| `hyperv_set_vm_network_adapter_team_mapping` | `Set-VMNetworkAdapterTeamMapping` | Configures team mapping settings for a virtual network adapter. |
| `hyperv_set_vm_network_adapter_vlan` | `Set-VMNetworkAdapterVlan` | Configures the virtual LAN settings for traffic through a virtual network adapter. |
| `hyperv_set_vm_processor` | `Set-VMProcessor` | Configures virtual processor settings for a virtual machine. |
| `hyperv_set_vm_remote_fx_3d_video_adapter` | `Set-VMRemoteFx3dVideoAdapter` | Configures the RemoteFX 3D video adapter of a virtual machine. |
| `hyperv_set_vm_replication` | `Set-VMReplication` | Modifies the replication settings of a virtual machine. |
| `hyperv_set_vm_replication_authorization_entry` | `Set-VMReplicationAuthorizationEntry` | Modifies an authorization entry on a Replica server. |
| `hyperv_set_vm_replication_server` | `Set-VMReplicationServer` | Configures a host as a Replica server. |
| `hyperv_set_vm_resource_pool` | `Set-VMResourcePool` | Sets the parent resource pool for a selected resource pool. |
| `hyperv_set_vm_san` | `Set-VMSan` | Configures a virtual SAN on one or more Hyper-V hosts. |
| `hyperv_set_vm_security` | `Set-VMSecurity` | Configures security settings for a virtual machine. |
| `hyperv_set_vm_security_policy` | `Set-VMSecurityPolicy` | Configures the security policy for a virtual machine. |
| `hyperv_set_vm_switch` | `Set-VMSwitch` | Configures a virtual switch. |
| `hyperv_set_vm_switch_extension_port_feature` | `Set-VMSwitchExtensionPortFeature` | Configures a feature on a virtual network adapter. |
| `hyperv_set_vm_switch_extension_switch_feature` | `Set-VMSwitchExtensionSwitchFeature` | Configures a feature on a virtual switch. |
| `hyperv_set_vm_switch_team` | `Set-VMSwitchTeam` | Configures a virtual switch team. |
| `hyperv_set_vm_video` | `Set-VMVideo` | Configures video settings for virtual machines. |

### Start / Stop / Suspend

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_start_vm` | `Start-VM` | Starts a virtual machine. |
| `hyperv_start_vm_failover` | `Start-VMFailover` | Starts failover on a virtual machine. |
| `hyperv_start_vm_initial_replication` | `Start-VMInitialReplication` | Starts replication of a virtual machine. |
| `hyperv_start_vm_trace` | `Start-VMTrace` | Starts tracing to a file. |
| `hyperv_stop_vm` | `Stop-VM` | Shuts down, turns off, or saves a virtual machine. |
| `hyperv_stop_vm_failover` | `Stop-VMFailover` | Stops failover of a virtual machine. |
| `hyperv_stop_vm_initial_replication` | `Stop-VMInitialReplication` | Stops an ongoing initial replication. |
| `hyperv_stop_vm_replication` | `Stop-VMReplication` | Cancels an ongoing virtual machine resynchronization. |
| `hyperv_stop_vm_trace` | `Stop-VMTrace` | Stops tracing to file. |
| `hyperv_suspend_vm` | `Suspend-VM` | Suspends (pauses) a virtual machine. |
| `hyperv_suspend_vm_replication` | `Suspend-VMReplication` | Suspends replication of a virtual machine. |

### Test / Update

| Tool | Cmdlet | Description |
|---|---|---|
| `hyperv_test_vhd` | `Test-VHD` | Tests a virtual hard disk for any problems that would make it unusable. |
| `hyperv_test_vm_network_adapter` | `Test-VMNetworkAdapter` | Tests connectivity between virtual machines. |
| `hyperv_test_vm_replication_connection` | `Test-VMReplicationConnection` | Tests the connection between a primary server and a Replica server. |
| `hyperv_update_vm_version` | `Update-VMVersion` | Updates the version of virtual machines. |

---

## Error Handling

| Layer | Behaviour |
|---|---|
| Startup elevation | Sidecar checks admin role. If not elevated, Rust prints `Error: Hyper-V MCP server must be run with Administrative privileges.` to stderr and exits with code 1. |
| Sidecar crash / hang | Rust detects missing response, kills the child, and restarts. Pending calls fail with `ToolError::Sidecar`. |
| Cmdlet errors | Sidecar catches exceptions and returns structured JSON (`Message`, `Category`, `FullyQualifiedErrorId`). Rust maps to `ToolError::PowerShell`. |
| Input validation | `serde` / `schemars` enforce schema at the MCP boundary. Custom validation returns `ToolError::InvalidInput`. |
| JSON parsing | Empty or whitespace sidecar output is coalesced to `"[]"` before parsing. Invalid JSON returns `ToolError::Json`. |

---

## Project Structure

```
hyperv-mcp/
├── Cargo.toml
├── sidecar/
│   └── hyperv_sidecar.ps1        # Embedded PowerShell sidecar
└── src/
    ├── main.rs                   # Startup, elevation check, serve(stdio())
    ├── server.rs                 # ServerHandler — tool/resource routing
    ├── tool.rs                   # HyperVTool trait, ToolContext, ToolError, register_tool!
    ├── sidecar.rs                # Spawn, restart, timeout, JSON-RPC framing
    ├── resources.rs              # hyperv:// resource handlers
    ├── ps_escape.rs              # PowerShell string escaping
    ├── mcp_content.rs            # MCP content helpers
    ├── config.rs                 # MCP config file load (%APPDATA%\hyperv-mcp\config.json)
    ├── credentials.rs            # DPAPI credential store (%APPDATA%\hyperv-mcp\credentials.json)
    ├── dpapi.rs                  # CryptProtectData / CryptUnprotectData helpers
    └── tools/
        ├── mod.rs                # pub mod declarations (orchestrator-managed)
        ├── get_vm.rs             # Reference tool
        ├── invoke_guest_command.rs
        ├── copy_to_guest.rs
        ├── copy_from_guest.rs
        ├── register_vm_credential.rs
        └── <verb>_<noun>.rs      # One file per cmdlet
```

---

## Crates

| Crate | Purpose |
|---|---|
| `rmcp` | Official Rust MCP SDK (`features = ["server", "macros"]`) |
| `tokio` | Async runtime and process management |
| `serde` / `serde_json` | Serialization for MCP payloads and sidecar JSON |
| `schemars` | JSON Schema generation from Rust structs |
| `inventory` | Link-time collection of `HyperVTool` implementations |
| `thiserror` | Structured error types |
| `tracing` / `tracing-subscriber` | Logging to stderr |
| `async-trait` | Async trait support |
| `windows` | DPAPI bindings (`Win32_Security_Cryptography`, `Win32_Foundation`) |

---

## Cmdlet Reference

Full documentation for all wrapped cmdlets: [https://learn.microsoft.com/en-us/powershell/module/hyper-v/](https://learn.microsoft.com/en-us/powershell/module/hyper-v/)
