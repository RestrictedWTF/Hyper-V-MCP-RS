use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostSupportedVersionInput {
    /// Name of the virtual machine configuration version. If omitted, returns all supported versions.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostSupportedVersionInfo {
    pub name: String,
    pub version: String,
    pub is_default: bool,
    pub is_readonly: bool,
    pub is_migration: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostSupportedVersionOutput {
    pub versions: Vec<VmHostSupportedVersionInfo>,
}

#[derive(Default)]
pub struct GetVmHostSupportedVersionTool;

#[async_trait]
impl HyperVTool for GetVmHostSupportedVersionTool {
    const NAME: &'static str = "hyperv_get_vm_host_supported_version";
    const DESCRIPTION: &'static str =
        "Returns a list of virtual machine configuration versions that are supported on a host.";
    type Input = GetVmHostSupportedVersionInput;
    type Output = GetVmHostSupportedVersionOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHostSupportedVersion".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name, \
             @{{N='Version';E={{$_.Version.ToString()}}}}, \
             IsDefault, IsReadOnly, IsMigration | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );
        // Note: Version is a System.Version object. It is forced to a string via a
        // calculated Select-Object property so serde_json sees a string value.

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let versions = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(versions.len());
        for version in versions {
            output.push(VmHostSupportedVersionInfo {
                name: version["Name"].as_str().unwrap_or_default().to_string(),
                version: version["Version"].as_str().unwrap_or_default().to_string(),
                is_default: version["IsDefault"].as_bool().unwrap_or_default(),
                is_readonly: version["IsReadOnly"].as_bool().unwrap_or_default(),
                is_migration: version["IsMigration"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmHostSupportedVersionOutput { versions: output })
    }
}

register_tool!(GetVmHostSupportedVersionTool);
