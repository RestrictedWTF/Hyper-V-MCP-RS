use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportVmSnapshotInput {
    /// Name of the checkpoint to export.
pub name: String,
    /// Name of the virtual machine that owns the checkpoint. Required if the checkpoint name is not unique across VMs.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Directory path to export the checkpoint into.
    pub path: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ExportVmSnapshotOutput {
    pub name: String,
    pub vm_name: Option<String>,
    pub export_path: String,
    pub computer_name: Option<String>,
}

#[derive(Default)]
pub struct ExportVmSnapshotTool;

#[async_trait]
impl HyperVTool for ExportVmSnapshotTool {
    const NAME: &'static str = "hyperv_export_vm_snapshot";
    const DESCRIPTION: &'static str = "Exports a virtual machine checkpoint to disk.";
    type Input = ExportVmSnapshotInput;
    type Output = ExportVmSnapshotOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "snapshot name is required".to_string(),
            ));
        }
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "export path is required".to_string(),
            ));
        }

        let mut args = vec![
            "Export-VMSnapshot".to_string(),
            format!("-Name '{}'", escape_ps_string(&input.name)),
            format!("-Path '{}'", escape_ps_string(&input.path)),
        ];
        if let Some(vm_name) = &input.vm_name {
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "{}" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        Ok(ExportVmSnapshotOutput {
            name: raw["Name"].as_str().unwrap_or(&input.name).to_string(),
            vm_name: input.vm_name,
            export_path: input.path,
            computer_name: input.computer_name,
        })
    }
}

register_tool!(ExportVmSnapshotTool);
