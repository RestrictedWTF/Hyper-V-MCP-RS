use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmBiosInput {
    /// Name of the Generation 1 virtual machine whose BIOS is to be configured.
    pub name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Boot device startup order, from first to last.
    #[serde(default, rename = "startupOrder")]
    pub startup_order: Option<Vec<String>>,
    /// Enable or disable Num Lock in the BIOS.
    #[serde(default, rename = "numLockEnabled")]
    pub num_lock_enabled: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmBiosInfo {
    pub vm_name: String,
    pub vm_id: String,
    pub computer_name: String,
    pub num_lock_enabled: bool,
    pub startup_order: Vec<String>,
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmBiosOutput {
    pub bios_entries: Vec<VmBiosInfo>,
}

#[derive(Default)]
pub struct SetVmBiosTool;

#[async_trait]
impl HyperVTool for SetVmBiosTool {
    const NAME: &'static str = "hyperv_set_vm_bios";
    const DESCRIPTION: &'static str = "Configures the BIOS of a Generation 1 virtual machine.";
    type Input = SetVmBiosInput;
    type Output = SetVmBiosOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMBios -VMName '{}'",
            escape_ps_string(&input.name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        if let Some(order) = &input.startup_order {
            let escaped: Vec<String> = order
                .iter()
                .map(|d| format!("'{}'", escape_ps_string(d)))
                .collect();
            args.push(format!("-StartupOrder @({})", escaped.join(",")));
        }

        if let Some(enabled) = input.num_lock_enabled {
            if enabled {
                args.push("-EnableNumLock".to_string());
            } else {
                args.push("-DisableNumLock".to_string());
            }
        }

        let ps = format!(
            "{} | ForEach-Object {{ [pscustomobject]@{{ \
             VMName = $_.VMName; \
             VMId = $_.VMId.ToString(); \
             ComputerName = $_.ComputerName; \
             NumLockEnabled = $_.NumLockEnabled; \
             StartupOrder = @($_.StartupOrder | ForEach-Object {{$_.ToString()}}); \
             IsDeleted = $_.IsDeleted }} }} | \
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

        let entries = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(entries.len());
        for entry in entries {
            let startup_order = match &entry["StartupOrder"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or_default().to_string())
                    .collect(),
                serde_json::Value::Object(obj) => obj
                    .get("value")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .map(|v| v.as_str().unwrap_or_default().to_string())
                            .collect()
                    })
                    .unwrap_or_default(),
                _ => Vec::new(),
            };

            output.push(VmBiosInfo {
                vm_name: entry["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: entry["VMId"].as_str().unwrap_or_default().to_string(),
                computer_name: entry["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                num_lock_enabled: entry["NumLockEnabled"].as_bool().unwrap_or_default(),
                startup_order,
                is_deleted: entry["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(SetVmBiosOutput {
            bios_entries: output,
        })
    }
}

register_tool!(SetVmBiosTool);
