use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmFibreChannelHbaSetInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "vmId")]
    pub vm_id: String,
    #[serde(rename = "sanName")]
    pub san_name: String,
    #[serde(rename = "worldWideNodeNameSetA")]
    pub world_wide_node_name_set_a: String,
    #[serde(rename = "worldWidePortNameSetA")]
    pub world_wide_port_name_set_a: String,
    #[serde(rename = "worldWideNodeNameSetB")]
    pub world_wide_node_name_set_b: String,
    #[serde(rename = "worldWidePortNameSetB")]
    pub world_wide_port_name_set_b: String,
    #[serde(rename = "isTemplate")]
    pub is_template: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmFibreChannelHbaInput {
    /// Name of the virtual machine.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the Fibre Channel HBA.
pub name: String,
    /// World-wide node name of address set A.
    #[serde(default, rename = "worldWideNodeNameSetA")]
    pub world_wide_node_name_set_a: Option<String>,
    /// World-wide port name of address set A.
    #[serde(default, rename = "worldWidePortNameSetA")]
    pub world_wide_port_name_set_a: Option<String>,
    /// World-wide node name of address set B.
    #[serde(default, rename = "worldWideNodeNameSetB")]
    pub world_wide_node_name_set_b: Option<String>,
    /// World-wide port name of address set B.
    #[serde(default, rename = "worldWidePortNameSetB")]
    pub world_wide_port_name_set_b: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmFibreChannelHbaOutput {
    pub adapters: Vec<VmFibreChannelHbaSetInfo>,
}


#[derive(Default)]
pub struct SetVmFibreChannelHbaTool;

#[async_trait]
impl HyperVTool for SetVmFibreChannelHbaTool {
    const NAME: &'static str = "hyperv_set_vm_fibre_channel_hba";
    const DESCRIPTION: &'static str = "Configures a Fibre Channel host bus adapter on a virtual machine.";
    type Input = SetVmFibreChannelHbaInput;
    type Output = SetVmFibreChannelHbaOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Set-VMFibreChannelHba".to_string()];
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".to_string()));
        }
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if let Some(world_wide_node_name_set_a) = &input.world_wide_node_name_set_a {
            if world_wide_node_name_set_a.trim().is_empty() {
                return Err(ToolError::InvalidInput("world_wide_node_name_set_a must not be empty when provided".to_string()));
            }
            args.push(format!("-WorldWideNodeNameSetA '{}'", escape_ps_string(world_wide_node_name_set_a)));
        }
        if let Some(world_wide_port_name_set_a) = &input.world_wide_port_name_set_a {
            if world_wide_port_name_set_a.trim().is_empty() {
                return Err(ToolError::InvalidInput("world_wide_port_name_set_a must not be empty when provided".to_string()));
            }
            args.push(format!("-WorldWidePortNameSetA '{}'", escape_ps_string(world_wide_port_name_set_a)));
        }
        if let Some(world_wide_node_name_set_b) = &input.world_wide_node_name_set_b {
            if world_wide_node_name_set_b.trim().is_empty() {
                return Err(ToolError::InvalidInput("world_wide_node_name_set_b must not be empty when provided".to_string()));
            }
            args.push(format!("-WorldWideNodeNameSetB '{}'", escape_ps_string(world_wide_node_name_set_b)));
        }
        if let Some(world_wide_port_name_set_b) = &input.world_wide_port_name_set_b {
            if world_wide_port_name_set_b.trim().is_empty() {
                return Err(ToolError::InvalidInput("world_wide_port_name_set_b must not be empty when provided".to_string()));
            }
            args.push(format!("-WorldWidePortNameSetB '{}'", escape_ps_string(world_wide_port_name_set_b)));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, VMName, VMId, SanName, WorldWideNodeNameSetA, WorldWidePortNameSetA, WorldWideNodeNameSetB, WorldWidePortNameSetB, IsTemplate | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmFibreChannelHbaSetInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                san_name: item["SanName"].as_str().unwrap_or_default().to_string(),
                world_wide_node_name_set_a: item["WorldWideNodeNameSetA"].as_str().unwrap_or_default().to_string(),
                world_wide_port_name_set_a: item["WorldWidePortNameSetA"].as_str().unwrap_or_default().to_string(),
                world_wide_node_name_set_b: item["WorldWideNodeNameSetB"].as_str().unwrap_or_default().to_string(),
                world_wide_port_name_set_b: item["WorldWidePortNameSetB"].as_str().unwrap_or_default().to_string(),
                is_template: item["IsTemplate"].as_bool().unwrap_or_default(),
            });
        }

        Ok(SetVmFibreChannelHbaOutput { adapters: output })

    }
}


register_tool!(SetVmFibreChannelHbaTool);
