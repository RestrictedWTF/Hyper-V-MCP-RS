use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmFibreChannelHbaInfo {
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
pub struct GetVmFibreChannelHbaInput {
    /// Name of the virtual machine.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmFibreChannelHbaOutput {
    pub adapters: Vec<VmFibreChannelHbaInfo>,
}

#[derive(Default)]
pub struct GetVmFibreChannelHbaTool;

#[async_trait]
impl HyperVTool for GetVmFibreChannelHbaTool {
    const NAME: &'static str = "hyperv_get_vm_fibre_channel_hba";
    const DESCRIPTION: &'static str =
        "Gets the Fibre Channel host bus adapters associated with one or more virtual machines.";
    type Input = GetVmFibreChannelHbaInput;
    type Output = GetVmFibreChannelHbaOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMFibreChannelHba".to_string()];
        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vm_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
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

        let ps = format!("{} | Select-Object Name, Id, VMName, VMId, SanName, @{{N='WorldWideNodeNameSetA';E={{$_.WorldWideNodeNameSetA.ToString()}}}}, @{{N='WorldWidePortNameSetA';E={{$_.WorldWidePortNameSetA.ToString()}}}}, @{{N='WorldWideNodeNameSetB';E={{$_.WorldWideNodeNameSetB.ToString()}}}}, @{{N='WorldWidePortNameSetB';E={{$_.WorldWidePortNameSetB.ToString()}}}}, IsTemplate | ConvertTo-Json -Compress -Depth 3", args.join(" "));

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
            output.push(VmFibreChannelHbaInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: item["VMId"].as_str().unwrap_or_default().to_string(),
                san_name: item["SanName"].as_str().unwrap_or_default().to_string(),
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
                is_template: item["IsTemplate"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmFibreChannelHbaOutput { adapters: output })
    }
}

register_tool!(GetVmFibreChannelHbaTool);
