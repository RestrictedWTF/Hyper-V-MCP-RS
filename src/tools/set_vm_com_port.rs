use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmComPortInput {
    /// Name of the virtual machine whose COM port is to be configured.
pub name: String,
    /// Id (1 or 2) of the COM port to be configured.
    pub number: i32,
    /// Named pipe path for the COM port, e.g. \\.\pipe\PipeName.
    #[serde(default)]
    pub path: Option<String>,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Debugger mode state: On or Off.
    #[serde(default, rename = "debuggerMode")]
    pub debugger_mode: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ComPortInfo {
    pub name: String,
    pub vm_name: String,
    pub number: i32,
    pub path: String,
    pub debugger_mode: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmComPortOutput {
    pub com_ports: Vec<ComPortInfo>,
}

#[derive(Default)]
pub struct SetVmComPortTool;

#[async_trait]
impl HyperVTool for SetVmComPortTool {
    const NAME: &'static str = "hyperv_set_vm_com_port";
    const DESCRIPTION: &'static str = "Configures the COM port of a virtual machine.";
    type Input = SetVmComPortInput;
    type Output = SetVmComPortOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.number != 1 && input.number != 2 {
            return Err(ToolError::InvalidInput(
                "COM port number must be 1 or 2".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMComPort -VMName '{}'",
            escape_ps_string(&input.name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push(format!("-Number {}", input.number));

        if let Some(path) = &input.path {
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }

        if let Some(mode) = &input.debugger_mode {
            args.push(format!("-DebuggerMode '{}'", escape_ps_string(mode)));
        }


        let ps = format!(
            "{} | Select-Object Name, VMName, Number, Path, \
             @{{N='DebuggerMode';E={{$_.DebuggerMode.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let ports = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(ports.len());
        for port in ports {
            output.push(ComPortInfo {
                name: port["Name"].as_str().unwrap_or_default().to_string(),
                vm_name: port["VMName"].as_str().unwrap_or_default().to_string(),
                number: port["Number"].as_i64().unwrap_or_default() as i32,
                path: port["Path"].as_str().unwrap_or_default().to_string(),
                debugger_mode: port["DebuggerMode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmComPortOutput { com_ports: output })
    }
}

register_tool!(SetVmComPortTool);
