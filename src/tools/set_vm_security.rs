use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSecuritySetInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "tpmEnabled")]
    pub tpm_enabled: bool,
    #[serde(rename = "ksdEnabled")]
    pub ksd_enabled: bool,
    pub shielded: bool,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmSecurityInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Enable or disable TPM.
    #[serde(default, rename = "tpmEnabled")]
    pub tpm_enabled: Option<bool>,
    /// Enable or disable KSD.
    #[serde(default, rename = "ksdEnabled")]
    pub ksd_enabled: Option<bool>,
    /// Enable or disable shielding.
    #[serde(default, rename = "shielded")]
    pub shielded: Option<bool>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmSecurityOutput {
    pub security: Vec<VmSecuritySetInfo>,
}


#[derive(Default)]
pub struct SetVmSecurityTool;

#[async_trait]
impl HyperVTool for SetVmSecurityTool {
    const NAME: &'static str = "hyperv_set_vm_security";
    const DESCRIPTION: &'static str = "Configures security settings for a virtual machine.";
    type Input = SetVmSecurityInput;
    type Output = SetVmSecurityOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMSecurity".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(tpm_enabled) = input.tpm_enabled {
            args.push(format!("-TpmEnabled ${}", tpm_enabled));
        }
        if let Some(ksd_enabled) = input.ksd_enabled {
            args.push(format!("-KsdEnabled ${}", ksd_enabled));
        }
        if let Some(shielded) = input.shielded {
            args.push(format!("-Shielded ${}", shielded));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object VMName, VMId, TpmEnabled, KsdEnabled, Shielded, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSecuritySetInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                tpm_enabled: item["TpmEnabled"].as_bool().unwrap_or_default(),
                ksd_enabled: item["KsdEnabled"].as_bool().unwrap_or_default(),
                shielded: item["Shielded"].as_bool().unwrap_or_default(),
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SetVmSecurityOutput { security: output })

    }
}


register_tool!(SetVmSecurityTool);
