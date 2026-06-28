use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmCompatibilityInfo {
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    pub compatible: bool,
    #[serde(rename = "incompatibilityMessages")]
    pub incompatibility_messages: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompareVmInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Path to the virtual machine configuration.
    #[serde(default, rename = "path")]
    pub path: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CompareVmOutput {
    pub results: Vec<VmCompatibilityInfo>,
}

#[derive(Default)]
pub struct CompareVmTool;

#[async_trait]
impl HyperVTool for CompareVmTool {
    const NAME: &'static str = "hyperv_compare_vm";
    const DESCRIPTION: &'static str = "Compares a virtual machine and a virtual machine host for compatibility, returning a compatibility report.";
    type Input = CompareVmInput;
    type Output = CompareVmOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Compare-VM".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vm_name must not be empty".to_string(),
            ));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(path) = &input.path {
            if path.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "path must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Path '{}'", escape_ps_string(path)));
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

        let ps = format!("{} | Select-Object VMName, VMId, ComputerName, Compatible, IncompatibilityMessages | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            let incompatibility_messages = match &item["IncompatibilityMessages"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                _ => Vec::new(),
            };
            output.push(VmCompatibilityInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                compatible: item["Compatible"].as_bool().unwrap_or_default(),
                incompatibility_messages,
            });
        }

        Ok(CompareVmOutput { results: output })
    }
}

register_tool!(CompareVmTool);
