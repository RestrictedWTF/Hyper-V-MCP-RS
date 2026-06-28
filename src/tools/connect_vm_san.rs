use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSanConnectInfo {
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
pub struct ConnectVmSanInput {
    /// Name of the virtual SAN.
pub name: String,
    /// World-wide node name.
    #[serde(rename = "worldWideNodeName")]
    pub world_wide_node_name: String,
    /// World-wide port name.
    #[serde(rename = "worldWidePortName")]
    pub world_wide_port_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ConnectVmSanOutput {
    pub sans: Vec<VmSanConnectInfo>,
}


#[derive(Default)]
pub struct ConnectVmSanTool;

#[async_trait]
impl HyperVTool for ConnectVmSanTool {
    const NAME: &'static str = "hyperv_connect_vm_san";
    const DESCRIPTION: &'static str = "Associates a host bus adapter with a virtual storage area network (SAN).";
    type Input = ConnectVmSanInput;
    type Output = ConnectVmSanOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Connect-VMSan".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if input.world_wide_node_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("world_wide_node_name must not be empty".to_string()));
        }
        args.push(format!("-WorldWideNodeName '{}'", escape_ps_string(&input.world_wide_node_name)));
        if input.world_wide_port_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("world_wide_port_name must not be empty".to_string()));
        }
        args.push(format!("-WorldWidePortName '{}'", escape_ps_string(&input.world_wide_port_name)));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, WorldWideNodeNameSetA, WorldWidePortNameSetA, WorldWideNodeNameSetB, WorldWidePortNameSetB, ComputerName | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmSanConnectInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                world_wide_node_name_set_a: item["WorldWideNodeNameSetA"].as_str().unwrap_or_default().to_string(),
                world_wide_port_name_set_a: item["WorldWidePortNameSetA"].as_str().unwrap_or_default().to_string(),
                world_wide_node_name_set_b: item["WorldWideNodeNameSetB"].as_str().unwrap_or_default().to_string(),
                world_wide_port_name_set_b: item["WorldWidePortNameSetB"].as_str().unwrap_or_default().to_string(),
                computer_name: item["ComputerName"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(ConnectVmSanOutput { sans: output })

    }
}


register_tool!(ConnectVmSanTool);
