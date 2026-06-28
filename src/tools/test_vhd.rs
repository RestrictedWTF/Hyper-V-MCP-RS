use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestVhdInput {
    /// Path to the virtual hard disk file to test.
    pub path: String,
    /// Hyper-V host to test against. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Test for SCSI persistent reservation support semantics.
    #[serde(default, rename = "supportPersistentReservations")]
    pub support_persistent_reservations: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TestVhdOutput {
    /// True if the virtual hard disk is usable; otherwise false.
    pub usable: bool,
}

#[derive(Default)]
pub struct TestVhdTool;

#[async_trait]
impl HyperVTool for TestVhdTool {
    const NAME: &'static str = "hyperv_test_vhd";
    const DESCRIPTION: &'static str =
        "Tests a virtual hard disk for any problems that would make it unusable.";
    type Input = TestVhdInput;
    type Output = TestVhdOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.path.trim().is_empty() {
            return Err(ToolError::InvalidInput("path is required".to_string()));
        }

        let mut args = vec!["Test-VHD".to_string()];
        args.push(format!("-Path '{}'", escape_ps_string(&input.path)));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if input.support_persistent_reservations.unwrap_or(false) {
            args.push("-SupportPersistentReservations".to_string());
        }

        let ps = format!("{} | ConvertTo-Json -Compress -Depth 3", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let usable = match raw {
            serde_json::Value::Bool(b) => b,
            serde_json::Value::Array(arr) if arr.is_empty() => {
                return Err(ToolError::Sidecar(
                    "empty response from sidecar".to_string(),
                ));
            }
            _ => {
                return Err(ToolError::Sidecar(format!(
                    "unexpected Test-VHD response: {}",
                    raw
                )));
            }
        };

        Ok(TestVhdOutput { usable })
    }
}

register_tool!(TestVhdTool);
