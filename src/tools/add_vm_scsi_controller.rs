use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmScsiControllerInput {
    /// Name of the virtual machine in which to add the SCSI controller.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ScsiControllerInfo {
    pub name: String,
    pub id: String,
    pub vm_name: String,
    pub controller_number: u32,
    pub number_of_queues: u32,
    pub queue_depth: u32,
    pub model: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmScsiControllerOutput {
    pub controllers: Vec<ScsiControllerInfo>,
}

#[derive(Default)]
pub struct AddVmScsiControllerTool;

#[async_trait]
impl HyperVTool for AddVmScsiControllerTool {
    const NAME: &'static str = "hyperv_add_vm_scsi_controller";
    const DESCRIPTION: &'static str = "Adds a SCSI controller in a virtual machine.";
    type Input = AddVmScsiControllerInput;
    type Output = AddVmScsiControllerOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Add-VMScsiController".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object Name, \
             @{{N='Id';E={{$_.Id.ToString()}}}}, \
             VMName, \
             ControllerNumber, \
             NumberOfQueues, \
             QueueDepth, \
             Model | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let controllers = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(controllers.len());
        for controller in controllers {
            output.push(ScsiControllerInfo {
                name: controller["Name"].as_str().unwrap_or_default().to_string(),
                id: controller["Id"].as_str().unwrap_or_default().to_string(),
                vm_name: controller["VMName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                controller_number: controller["ControllerNumber"].as_u64().unwrap_or_default()
                    as u32,
                number_of_queues: controller["NumberOfQueues"].as_u64().unwrap_or_default() as u32,
                queue_depth: controller["QueueDepth"].as_u64().unwrap_or_default() as u32,
                model: controller["Model"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(AddVmScsiControllerOutput {
            controllers: output,
        })
    }
}

register_tool!(AddVmScsiControllerTool);
