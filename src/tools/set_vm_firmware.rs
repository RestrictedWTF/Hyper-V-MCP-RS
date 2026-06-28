use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmFirmwareInput {
    /// Name of the virtual machine whose firmware configuration is to be set.
pub name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Enables or disables Secure Boot for Generation 2 virtual machines: On or Off.
    #[serde(default, rename = "enableSecureBoot")]
    pub enable_secure_boot: Option<String>,
    /// Secure Boot template to apply (e.g., MicrosoftWindows, MicrosoftUEFICertificateAuthority).
    #[serde(default, rename = "secureBootTemplate")]
    pub secure_boot_template: Option<String>,
    /// Preferred network boot protocol: IPv4, IPv6, or IPv4IPv6.
    #[serde(default, rename = "preferredNetworkBootProtocol")]
    pub preferred_network_boot_protocol: Option<String>,
    /// Allows a legacy network adapter to be used for network boot.
    #[serde(default, rename = "allowLegacyNetworkAdapter")]
    pub allow_legacy_network_adapter: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmFirmwareConfig {
    pub vm_name: String,
    pub secure_boot: String,
    pub secure_boot_template: String,
    pub preferred_network_boot_protocol: String,
    pub allow_legacy_network_adapter: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmFirmwareOutput {
    pub firmware: Vec<VmFirmwareConfig>,
}

#[derive(Default)]
pub struct SetVmFirmwareTool;

#[async_trait]
impl HyperVTool for SetVmFirmwareTool {
    const NAME: &'static str = "hyperv_set_vm_firmware";
    const DESCRIPTION: &'static str = "Sets the firmware configuration of a virtual machine.";
    type Input = SetVmFirmwareInput;
    type Output = SetVmFirmwareOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMFirmware -VMName '{}'",
            escape_ps_string(&input.name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(state) = &input.enable_secure_boot {
            args.push(format!("-EnableSecureBoot '{}'", escape_ps_string(state)));
        }
        if let Some(template) = &input.secure_boot_template {
            args.push(format!(
                "-SecureBootTemplate '{}'",
                escape_ps_string(template)
            ));
        }
        if let Some(protocol) = &input.preferred_network_boot_protocol {
            args.push(format!(
                "-PreferredNetworkBootProtocol '{}'",
                escape_ps_string(protocol)
            ));
        }
        if let Some(allow) = input.allow_legacy_network_adapter {
            args.push(format!("-AllowLegacyNetworkAdapter ${}", allow));
        }


        let ps = format!(
            "{} | Select-Object VMName, \
             @{{N='SecureBoot';E={{$_.SecureBoot.ToString()}}}}, \
             @{{N='SecureBootTemplate';E={{$_.SecureBootTemplate.ToString()}}}}, \
             @{{N='PreferredNetworkBootProtocol';E={{$_.PreferredNetworkBootProtocol.ToString()}}}}, \
             AllowLegacyNetworkAdapter | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let firmware_list = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(firmware_list.len());
        for fw in firmware_list {
            output.push(VmFirmwareConfig {
                vm_name: fw["VMName"].as_str().unwrap_or_default().to_string(),
                secure_boot: fw["SecureBoot"].as_str().unwrap_or_default().to_string(),
                secure_boot_template: fw["SecureBootTemplate"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                preferred_network_boot_protocol: fw["PreferredNetworkBootProtocol"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                allow_legacy_network_adapter: fw["AllowLegacyNetworkAdapter"]
                    .as_bool()
                    .unwrap_or_default(),
            });
        }

        Ok(SetVmFirmwareOutput { firmware: output })
    }
}

register_tool!(SetVmFirmwareTool);
