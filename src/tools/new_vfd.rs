use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NewVfdInput {
    /// Path to the new virtual floppy disk file to create.
    pub path: String,
    /// Hyper-V host on which to create the virtual floppy disk. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VfdInfo {
    pub full_name: String,
    pub name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct NewVfdOutput {
    pub disks: Vec<VfdInfo>,
}

#[derive(Default)]
pub struct NewVfdTool;

#[async_trait]
impl HyperVTool for NewVfdTool {
    const NAME: &'static str = "hyperv_new_vfd";
    const DESCRIPTION: &'static str = "Creates a virtual floppy disk.";
    type Input = NewVfdInput;
    type Output = NewVfdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Path must not be empty".to_string(),
            ));
        }

        let mut args = vec!["New-VFD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object FullName, Name, Length | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let disks = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(disks.len());
        for disk in disks {
            output.push(VfdInfo {
                full_name: disk["FullName"].as_str().unwrap_or_default().to_string(),
                name: disk["Name"].as_str().unwrap_or_default().to_string(),
                size_bytes: disk["Length"].as_u64().unwrap_or_default(),
            });
        }

        Ok(NewVfdOutput { disks: output })
    }
}

register_tool!(NewVfdTool);
