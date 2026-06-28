use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DisconnectVmSanInput {
    /// Name of the virtual storage area network (SAN) from which to remove the host bus adapter.
pub name: String,
    /// World wide node name of the host bus adapter to remove.
    #[serde(rename = "worldWideNodeName")]
    pub world_wide_node_name: Vec<String>,
    /// World wide port name of the host bus adapter to remove.
    #[serde(rename = "worldWidePortName")]
    pub world_wide_port_name: Vec<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSanInfo {
    pub name: String,
    pub note: Option<String>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "worldWideNodeName")]
    pub world_wide_node_name: Vec<String>,
    #[serde(rename = "worldWidePortName")]
    pub world_wide_port_name: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct DisconnectVmSanOutput {
    /// Virtual SANs from which the host bus adapter was removed.
    pub sans: Vec<VmSanInfo>,
}

#[derive(Default)]
pub struct DisconnectVmSanTool;

#[async_trait]
impl HyperVTool for DisconnectVmSanTool {
    const NAME: &'static str = "hyperv_disconnect_vm_san";
    const DESCRIPTION: &'static str =
        "Removes a host bus adapter from a virtual storage area network (SAN).";
    type Input = DisconnectVmSanInput;
    type Output = DisconnectVmSanOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(ToolError::InvalidInput(
                "SAN name must not be empty".to_string(),
            ));
        }

        let node_names: Vec<&str> = input
            .world_wide_node_name
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if node_names.is_empty() {
            return Err(ToolError::InvalidInput(
                "worldWideNodeName must contain at least one non-empty value".to_string(),
            ));
        }

        let port_names: Vec<&str> = input
            .world_wide_port_name
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if port_names.is_empty() {
            return Err(ToolError::InvalidInput(
                "worldWidePortName must contain at least one non-empty value".to_string(),
            ));
        }

        if node_names.len() != port_names.len() {
            return Err(ToolError::InvalidInput(
                "worldWideNodeName and worldWidePortName must have the same number of entries"
                    .to_string(),
            ));
        }

        let mut args = vec!["Disconnect-VMSan".to_string()];
        args.push(format!("-Name '{}'", escape_ps_string(name)));

        let node_array = node_names
            .iter()
            .map(|s| format!("'{}'", escape_ps_string(s)))
            .collect::<Vec<_>>()
            .join(", ");
        args.push(format!("-WorldWideNodeName @({})", node_array));

        let port_array = port_names
            .iter()
            .map(|s| format!("'{}'", escape_ps_string(s)))
            .collect::<Vec<_>>()
            .join(", ");
        args.push(format!("-WorldWidePortName @({})", port_array));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-PassThru".to_string());
        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object \
             Name, Note, ComputerName, \
             WorldWideNodeName, WorldWidePortName | \
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

        let sans = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(sans.len());
        for san in sans {
            output.push(VmSanInfo {
                name: san["Name"].as_str().unwrap_or_default().to_string(),
                note: san["Note"].as_str().map(String::from),
                computer_name: san["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_node_name: strings_from_value(&san["WorldWideNodeName"]),
                world_wide_port_name: strings_from_value(&san["WorldWidePortName"]),
            });
        }

        Ok(DisconnectVmSanOutput { sans: output })
    }
}

fn strings_from_value(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Null => Vec::new(),
        other => vec![other.to_string()],
    }
}

register_tool!(DisconnectVmSanTool);
