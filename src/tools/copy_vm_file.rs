use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CopyVmFileInput {
    /// Name of the virtual machine to copy the file to.
pub name: String,
    /// Path to the source file on the Hyper-V host.
    #[serde(rename = "sourcePath")]
    pub source_path: String,
    /// Destination path inside the virtual machine.
    #[serde(rename = "destinationPath")]
    pub destination_path: String,
    /// Source of the file. Defaults to Host.
    #[serde(default = "default_file_source")]
    pub file_source: String,
    /// Create the full destination folder path if it does not exist.
    #[serde(default)]
    pub create_full_path: bool,
    /// Overwrite the destination file if it already exists.
    #[serde(default)]
    pub force: bool,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

fn default_file_source() -> String {
    "Host".to_string()
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct CopyVmFileOutput {
    /// True if the file was copied successfully.
    pub success: bool,
    /// Name of the virtual machine that received the file.
pub name: String,
    /// Destination path inside the virtual machine.
    #[serde(rename = "destinationPath")]
    pub destination_path: String,
}

#[derive(Default)]
pub struct CopyVmFileTool;

#[async_trait]
impl HyperVTool for CopyVmFileTool {
    const NAME: &'static str = "hyperv_copy_vm_file";
    const DESCRIPTION: &'static str = "Copies a file to a virtual machine.";
    type Input = CopyVmFileInput;
    type Output = CopyVmFileOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.source_path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "SourcePath must not be empty".to_string(),
            ));
        }
        if input.destination_path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "DestinationPath must not be empty".to_string(),
            ));
        }
        if input.file_source.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "FileSource must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Copy-VMFile".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        args.push(format!(
            "-SourcePath '{}'",
            escape_ps_string(&input.source_path)
        ));
        args.push(format!(
            "-DestinationPath '{}'",
            escape_ps_string(&input.destination_path)
        ));
        args.push(format!(
            "-FileSource '{}'",
            escape_ps_string(&input.file_source)
        ));
        if input.create_full_path {
            args.push("-CreateFullPath".to_string());
        }
        if input.force {
            args.push("-Force".to_string());
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = args.join(" ");

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        if let serde_json::Value::Array(arr) = raw {
            if !arr.is_empty() {
                return Err(ToolError::InvalidInput(
                    "Unexpected output from Copy-VMFile".to_string(),
                ));
            }
        }

        Ok(CopyVmFileOutput {
            success: true,
            name: input.name,
            destination_path: input.destination_path,
        })
    }
}

register_tool!(CopyVmFileTool);
