use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmBiosInput {
    /// Name of the virtual machine whose BIOS is to be retrieved.
    #[serde(default)]
    pub vm_name: Option<String>,
    /// Name of the BIOS to retrieve. If omitted, returns the BIOS of the specified VM.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmBiosInfo {
    pub vm_name: String,
    pub vm_id: String,
    pub name: String,
    pub computer_name: String,
    pub startup_order: Vec<String>,
    pub num_lock_enabled: bool,
    pub pause_after_boot_failure: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmBiosOutput {
    pub bios: Vec<VmBiosInfo>,
}

#[derive(Default)]
pub struct GetVmBiosTool;

#[async_trait]
impl HyperVTool for GetVmBiosTool {
    const NAME: &'static str = "hyperv_get_vm_bios";
    const DESCRIPTION: &'static str = "Gets the BIOS of a virtual machine or snapshot.";
    type Input = GetVmBiosInput;
    type Output = GetVmBiosOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMBios".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "BIOS name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object VMName, VMId, Name, ComputerName, \
             @{{N='StartupOrder';E={{$_.StartupOrder | ForEach-Object {{ $_.ToString() }} }}}}, \
             NumLockEnabled, PauseAfterBootFailure | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let bios_entries = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(bios_entries.len());
        for entry in bios_entries {
            let startup_order = match &entry["StartupOrder"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                serde_json::Value::String(s) => vec![s.clone()],
                _ => Vec::new(),
            };

            output.push(VmBiosInfo {
                vm_name: entry["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: entry["VMId"].as_str().unwrap_or_default().to_string(),
                name: entry["Name"].as_str().unwrap_or_default().to_string(),
                computer_name: entry["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                startup_order,
                num_lock_enabled: entry["NumLockEnabled"].as_bool().unwrap_or_default(),
                pause_after_boot_failure: entry["PauseAfterBootFailure"]
                    .as_bool()
                    .unwrap_or_default(),
            });
        }

        Ok(GetVmBiosOutput { bios: output })
    }
}

register_tool!(GetVmBiosTool);
