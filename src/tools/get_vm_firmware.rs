use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmFirmwareInput {
    /// Name of the virtual machine whose firmware configuration is to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmFirmwareInfo {
    pub name: String,
    pub computer_name: String,
    pub vm_name: String,
    pub vm_id: String,
    pub secure_boot: String,
    pub secure_boot_template: String,
    pub preferred_network_boot_protocol: String,
    pub boot_order: Vec<String>,
    pub pause_after_boot_failure: String,
    pub lock_on_disconnect: String,
    pub console_mode: String,
    pub allow_legacy_network_adapter: String,
    pub pxe_ipv4_boot: String,
    pub pxe_ipv6_boot: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmFirmwareOutput {
    pub firmware_configs: Vec<VmFirmwareInfo>,
}

#[derive(Default)]
pub struct GetVmFirmwareTool;

#[async_trait]
impl HyperVTool for GetVmFirmwareTool {
    const NAME: &'static str = "hyperv_get_vm_firmware";
    const DESCRIPTION: &'static str = "Gets the firmware configuration of a virtual machine.";
    type Input = GetVmFirmwareInput;
    type Output = GetVmFirmwareOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMFirmware".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object \
             Name, ComputerName, VMName, VMId, \
             @{{N='SecureBoot';E={{$_.SecureBoot.ToString()}}}}, \
             @{{N='SecureBootTemplate';E={{$_.SecureBootTemplate.ToString()}}}}, \
             @{{N='PreferredNetworkBootProtocol';E={{$_.PreferredNetworkBootProtocol.ToString()}}}}, \
             @{{N='BootOrder';E={{ @($_.BootOrder | ForEach-Object {{ $_.ToString() }}) }}}}, \
             @{{N='PauseAfterBootFailure';E={{$_.PauseAfterBootFailure.ToString()}}}}, \
             @{{N='LockOnDisconnect';E={{$_.LockOnDisconnect.ToString()}}}}, \
             @{{N='ConsoleMode';E={{$_.ConsoleMode.ToString()}}}}, \
             @{{N='AllowLegacyNetworkAdapter';E={{$_.AllowLegacyNetworkAdapter.ToString()}}}}, \
             @{{N='PxeIpv4Boot';E={{$_.PxeIpv4Boot.ToString()}}}}, \
             @{{N='PxeIpv6Boot';E={{$_.PxeIpv6Boot.ToString()}}}} | \
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

        let configs = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(configs.len());
        for config in configs {
            let boot_order = match &config["BootOrder"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                serde_json::Value::String(s) => {
                    if s.trim().is_empty() {
                        Vec::new()
                    } else {
                        vec![s.clone()]
                    }
                }
                serde_json::Value::Null => Vec::new(),
                _ => Vec::new(),
            };

            output.push(VmFirmwareInfo {
                name: config["Name"].as_str().unwrap_or_default().to_string(),
                computer_name: config["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_name: config["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: config["VMId"].as_str().unwrap_or_default().to_string(),
                secure_boot: config["SecureBoot"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                secure_boot_template: config["SecureBootTemplate"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                preferred_network_boot_protocol: config["PreferredNetworkBootProtocol"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                boot_order,
                pause_after_boot_failure: config["PauseAfterBootFailure"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                lock_on_disconnect: config["LockOnDisconnect"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                console_mode: config["ConsoleMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                allow_legacy_network_adapter: config["AllowLegacyNetworkAdapter"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                pxe_ipv4_boot: config["PxeIpv4Boot"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                pxe_ipv6_boot: config["PxeIpv6Boot"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmFirmwareOutput {
            firmware_configs: output,
        })
    }
}

register_tool!(GetVmFirmwareTool);
