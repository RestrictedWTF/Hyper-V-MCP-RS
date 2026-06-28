use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmScsiControllerInput {
    /// Name of the virtual machine whose SCSI controllers are to be retrieved.
    #[serde(default, rename = "vmName")]
    pub vm_name: Option<String>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmScsiControllerInfo {
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
pub struct GetVmScsiControllerOutput {
    pub controllers: Vec<VmScsiControllerInfo>,
}

#[derive(Default)]
pub struct GetVmScsiControllerTool;

#[async_trait]
impl HyperVTool for GetVmScsiControllerTool {
    const NAME: &'static str = "hyperv_get_vm_scsi_controller";
    const DESCRIPTION: &'static str = "Gets the SCSI controllers of a virtual machine or snapshot.";
    type Input = GetVmScsiControllerInput;
    type Output = GetVmScsiControllerOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMScsiController".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

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

        let mut output = Vec::with_capacity(controllers.len());
        for controller in controllers {
            output.push(VmScsiControllerInfo {
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

        Ok(GetVmScsiControllerOutput {
            controllers: output,
        })
    }
}

register_tool!(GetVmScsiControllerTool);
