use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveVmScsiControllerInput {
    /// Name of the virtual machine whose SCSI controller is being removed.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Number of the SCSI controller to remove.
    #[serde(rename = "controllerNumber")]
    pub controller_number: u32,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemovedScsiControllerInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "vmName")]
    pub vm_name: String,
    #[serde(rename = "controllerNumber")]
    pub controller_number: u32,
    #[serde(rename = "numberOfQueues")]
    pub number_of_queues: u32,
    #[serde(rename = "queueDepth")]
    pub queue_depth: u32,
    pub model: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RemoveVmScsiControllerOutput {
    /// SCSI controllers that were removed.
    pub removed: Vec<RemovedScsiControllerInfo>,
}

#[derive(Default)]
pub struct RemoveVmScsiControllerTool;

#[async_trait]
impl HyperVTool for RemoveVmScsiControllerTool {
    const NAME: &'static str = "hyperv_remove_vm_scsi_controller";
    const DESCRIPTION: &'static str = "Removes a SCSI controller from a virtual machine.";
    type Input = RemoveVmScsiControllerInput;
    type Output = RemoveVmScsiControllerOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "vmName must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Remove-VMScsiController".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));
        args.push(format!("-ControllerNumber {}", input.controller_number));

        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computerName must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push("-Confirm:$false".to_string());

        let ps = format!(
            "{} | Select-Object Name, \
             @{{N='Id';E={{$_.Id.ToString()}}}}, \
             VMName, \
             ControllerNumber, \
             NumberOfQueues, \
             QueueDepth, \
             @{{N='Model';E={{$_.Model.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
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

        let mut removed = Vec::with_capacity(controllers.len());
        for controller in controllers {
            removed.push(RemovedScsiControllerInfo {
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

        Ok(RemoveVmScsiControllerOutput { removed })
    }
}

register_tool!(RemoveVmScsiControllerTool);
