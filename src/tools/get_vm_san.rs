use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSanInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "worldWideNodeNameSetA")]
    pub world_wide_node_name_set_a: String,
    #[serde(rename = "worldWidePortNameSetA")]
    pub world_wide_port_name_set_a: String,
    #[serde(rename = "worldWideNodeNameSetB")]
    pub world_wide_node_name_set_b: String,
    #[serde(rename = "worldWidePortNameSetB")]
    pub world_wide_port_name_set_b: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmSanInput {
    /// Name of the virtual SAN.
    #[serde(default, rename = "name")]
    pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmSanOutput {
    pub sans: Vec<VmSanInfo>,
}

#[derive(Default)]
pub struct GetVmSanTool;

#[async_trait]
impl HyperVTool for GetVmSanTool {
    const NAME: &'static str = "hyperv_get_vm_san";
    const DESCRIPTION: &'static str =
        "Gets the available virtual machine storage area networks on a Hyper-V host or hosts.";
    type Input = GetVmSanInput;
    type Output = GetVmSanOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMSan".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
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

        let ps = format!("{} | Select-Object Name, Id, @{{N='WorldWideNodeNameSetA';E={{$_.WorldWideNodeNameSetA.ToString()}}}}, @{{N='WorldWidePortNameSetA';E={{$_.WorldWidePortNameSetA.ToString()}}}}, @{{N='WorldWideNodeNameSetB';E={{$_.WorldWideNodeNameSetB.ToString()}}}}, @{{N='WorldWidePortNameSetB';E={{$_.WorldWidePortNameSetB.ToString()}}}}, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSanInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                world_wide_node_name_set_a: item["WorldWideNodeNameSetA"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_port_name_set_a: item["WorldWidePortNameSetA"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_node_name_set_b: item["WorldWideNodeNameSetB"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_port_name_set_b: item["WorldWidePortNameSetB"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: item["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmSanOutput { sans: output })
    }
}

register_tool!(GetVmSanTool);
