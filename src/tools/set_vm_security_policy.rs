use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSecurityPolicySetInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "encryptionType")]
    pub encryption_type: String,
    #[serde(rename = "shieldingDataFilePath")]
    pub shielding_data_file_path: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmSecurityPolicyInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Encryption type.
    #[serde(default, rename = "encryptionType")]
    pub encryption_type: Option<String>,
    /// Path to the shielding data file.
    #[serde(default, rename = "shieldingDataFilePath")]
    pub shielding_data_file_path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmSecurityPolicyOutput {
    pub policies: Vec<VmSecurityPolicySetInfo>,
}

#[derive(Default)]
pub struct SetVmSecurityPolicyTool;

#[async_trait]
impl HyperVTool for SetVmSecurityPolicyTool {
    const NAME: &'static str = "hyperv_set_vm_security_policy";
    const DESCRIPTION: &'static str = "Configures the security policy for a virtual machine.";
    type Input = SetVmSecurityPolicyInput;
    type Output = SetVmSecurityPolicyOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMSecurityPolicy".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_name must not be empty".to_string(),
            ));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(encryption_type) = &input.encryption_type {
            if encryption_type.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "encryption_type must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-EncryptionType '{}'",
                escape_ps_string(encryption_type)
            ));
        }
        if let Some(shielding_data_file_path) = &input.shielding_data_file_path {
            if shielding_data_file_path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "shielding_data_file_path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ShieldingDataFilePath '{}'",
                escape_ps_string(shielding_data_file_path)
            ));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = format!("{} | Select-Object VMName, VMId, EncryptionType, ShieldingDataFilePath, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSecurityPolicySetInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                encryption_type: item["EncryptionType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                shielding_data_file_path: item["ShieldingDataFilePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmSecurityPolicyOutput { policies: output })
    }
}

register_tool!(SetVmSecurityPolicyTool);
