use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmKeyProtectorSetInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "keyProtector")]
    pub key_protector: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmKeyProtectorInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Key protector string.
    #[serde(rename = "keyProtector")]
    pub key_protector: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmKeyProtectorOutput {
    pub protectors: Vec<VmKeyProtectorSetInfo>,
}


#[derive(Default)]
pub struct SetVmKeyProtectorTool;

#[async_trait]
impl HyperVTool for SetVmKeyProtectorTool {
    const NAME: &'static str = "hyperv_set_vm_key_protector";
    const DESCRIPTION: &'static str = "Configures a key protector for a virtual machine.";
    type Input = SetVmKeyProtectorInput;
    type Output = SetVmKeyProtectorOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMKeyProtector".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if input.key_protector.trim().is_empty() {
            return Err(ToolError::InvalidInput("key_protector must not be empty".to_string()));
        }
        args.push(format!("-KeyProtector '{}'", escape_ps_string(&input.key_protector)));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object VMName, VMId, @{{N='KeyProtector';E={{$_.KeyProtector.ToString()}}}} | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmKeyProtectorSetInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                key_protector: item["KeyProtector"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SetVmKeyProtectorOutput { protectors: output })

    }
}


register_tool!(SetVmKeyProtectorTool);
