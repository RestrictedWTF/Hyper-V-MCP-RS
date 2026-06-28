use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmComPortInput {
    /// Name of the virtual machine whose COM ports are to be retrieved.
    #[serde(rename = "vmName")]
    pub vm_name: Option<String>,
    /// Id (1 or 2) of the COM port to retrieve. If omitted, all COM ports are returned.
    #[serde(default)]
    pub number: Option<u32>,
    /// Hyper-V host to query. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ComPortInfo {
    pub name: String,
    pub number: u32,
    pub path: String,
    pub debugger_mode: String,
    pub vm_name: String,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmComPortOutput {
    pub com_ports: Vec<ComPortInfo>,
}

#[derive(Default)]
pub struct GetVmComPortTool;

#[async_trait]
impl HyperVTool for GetVmComPortTool {
    const NAME: &'static str = "hyperv_get_vm_com_port";
    const DESCRIPTION: &'static str = "Gets the COM ports of a virtual machine or snapshot.";
    type Input = GetVmComPortInput;
    type Output = GetVmComPortOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMComPort".to_string()];

        if let Some(vm_name) = &input.vm_name {
            if vm_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "VM name must not be empty".to_string(),
                ));
            }
            args.push(format!("-VMName '{}'", escape_ps_string(vm_name)));
        }
        if let Some(number) = input.number {
            args.push(format!("-Number {}", number));
        }
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        let ps = format!(
            "{} | Select-Object Name, Number, Path, \
             @{{N='DebuggerMode';E={{$_.DebuggerMode.ToString()}}}}, \
             VMName, ComputerName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );
        // Note: DebuggerMode is a .NET enum object. It is forced to a string via a
        // calculated Select-Object property so serde_json sees a string value.

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let com_ports = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(com_ports.len());
        for com_port in com_ports {
            output.push(ComPortInfo {
                name: com_port["Name"].as_str().unwrap_or_default().to_string(),
                number: com_port["Number"].as_u64().unwrap_or_default() as u32,
                path: com_port["Path"].as_str().unwrap_or_default().to_string(),
                debugger_mode: com_port["DebuggerMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                vm_name: com_port["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: com_port["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(GetVmComPortOutput { com_ports: output })
    }
}

register_tool!(GetVmComPortTool);
