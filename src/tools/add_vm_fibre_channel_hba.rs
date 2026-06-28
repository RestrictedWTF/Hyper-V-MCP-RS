use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmFibreChannelHbaInput {
    /// Name of the virtual machine to which the Fibre Channel HBA is added.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the virtual storage area network (SAN) to associate with the adapter.
    #[serde(rename = "sanName")]
    pub san_name: String,
    /// Generate world-wide names automatically. Cannot be combined with manual WWN fields.
    #[serde(default, rename = "generateWwn")]
    pub generate_wwn: Option<bool>,
    /// World-wide node name of address set A (manual WWN set).
    #[serde(default, rename = "worldWideNodeNameSetA")]
    pub world_wide_node_name_set_a: Option<String>,
    /// World-wide port name of address set A (manual WWN set).
    #[serde(default, rename = "worldWidePortNameSetA")]
    pub world_wide_port_name_set_a: Option<String>,
    /// World-wide node name of address set B (manual WWN set).
    #[serde(default, rename = "worldWideNodeNameSetB")]
    pub world_wide_node_name_set_b: Option<String>,
    /// World-wide port name of address set B (manual WWN set).
    #[serde(default, rename = "worldWidePortNameSetB")]
    pub world_wide_port_name_set_b: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

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

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmFibreChannelHbaOutput {
    pub adapters: Vec<VmFibreChannelHbaInfo>,
}

#[derive(Default)]
pub struct AddVmFibreChannelHbaTool;

#[async_trait]
impl HyperVTool for AddVmFibreChannelHbaTool {
    const NAME: &'static str = "hyperv_add_vm_fibre_channel_hba";
    const DESCRIPTION: &'static str =
        "Adds a virtual Fibre Channel host bus adapter to a virtual machine.";
    type Input = AddVmFibreChannelHbaInput;
    type Output = AddVmFibreChannelHbaOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.san_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "SAN name must not be empty".to_string(),
            ));
        }

        let manual_fields = [
            &input.world_wide_node_name_set_a,
            &input.world_wide_port_name_set_a,
            &input.world_wide_node_name_set_b,
            &input.world_wide_port_name_set_b,
        ];
        let has_manual = manual_fields.iter().any(|f| f.is_some());

        if input.generate_wwn == Some(true) && has_manual {
            return Err(ToolError::InvalidInput(
                "generate_wwn cannot be combined with manual WWN fields".to_string(),
            ));
        }

        if has_manual {
            for (label, value) in [
                ("WorldWideNodeNameSetA", &input.world_wide_node_name_set_a),
                ("WorldWidePortNameSetA", &input.world_wide_port_name_set_a),
                ("WorldWideNodeNameSetB", &input.world_wide_node_name_set_b),
                ("WorldWidePortNameSetB", &input.world_wide_port_name_set_b),
            ] {
                if value.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                    return Err(ToolError::InvalidInput(format!(
                        "{} must not be empty when providing manual WWN fields",
                        label
                    )));
                }
            }
        }

        let mut args = vec![format!(
            "Add-VMFibreChannelHba -VMName '{}' -SanName '{}'",
            escape_ps_string(&input.vm_name),
            escape_ps_string(&input.san_name)
        )];

        if input.generate_wwn == Some(true) {
            args.push("-GenerateWwn".to_string());
        }

        if let Some(wwnn_a) = &input.world_wide_node_name_set_a {
            args.push(format!(
                "-WorldWideNodeNameSetA '{}'",
                escape_ps_string(wwnn_a)
            ));
        }
        if let Some(wwpn_a) = &input.world_wide_port_name_set_a {
            args.push(format!(
                "-WorldWidePortNameSetA '{}'",
                escape_ps_string(wwpn_a)
            ));
        }
        if let Some(wwnn_b) = &input.world_wide_node_name_set_b {
            args.push(format!(
                "-WorldWideNodeNameSetB '{}'",
                escape_ps_string(wwnn_b)
            ));
        }
        if let Some(wwpn_b) = &input.world_wide_port_name_set_b {
            args.push(format!(
                "-WorldWidePortNameSetB '{}'",
                escape_ps_string(wwpn_b)
            ));
        }

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "ComputerName must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }


        let ps = format!(
            "{} | Select-Object \
             Name, \
             @{{N='Id';E={{$_.Id.ToString()}}}}, \
             VMName, \
             @{{N='VMId';E={{$_.VMId.ToString()}}}}, \
             SanName, \
             WorldWideNodeNameSetA, \
             WorldWidePortNameSetA, \
             WorldWideNodeNameSetB, \
             WorldWidePortNameSetB, \
             IsTemplate | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let adapters = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(adapters.len());
        for adapter in adapters {
            output.push(VmFibreChannelHbaInfo {
                name: adapter["Name"].as_str().unwrap_or_default().to_string(),
                id: adapter["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: adapter["VMName"].as_str().unwrap_or_default().to_string(),
                vm_id: adapter["VMId"].as_str().unwrap_or_default().to_string(),
                san_name: adapter["SanName"].as_str().unwrap_or_default().to_string(),
                world_wide_node_name_set_a: adapter["WorldWideNodeNameSetA"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_port_name_set_a: adapter["WorldWidePortNameSetA"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_node_name_set_b: adapter["WorldWideNodeNameSetB"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                world_wide_port_name_set_b: adapter["WorldWidePortNameSetB"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_template: adapter["IsTemplate"].as_bool().unwrap_or_default(),
            });
        }

        Ok(AddVmFibreChannelHbaOutput { adapters: output })
    }
}

register_tool!(AddVmFibreChannelHbaTool);
