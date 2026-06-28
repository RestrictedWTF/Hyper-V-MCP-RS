use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmSwitchInput {
    /// Name of the virtual switch to configure.
    pub name: String,
    /// Hyper-V host on which the virtual switch resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Converts the switch to Internal or Private. Cannot be used with net_adapter_name or net_adapter_interface_description.
    #[serde(default, rename = "switchType")]
    pub switch_type: Option<String>,
    /// Allows the management operating system to share the physical adapter bound to the switch.
    #[serde(default, rename = "allowManagementOS")]
    pub allow_management_os: Option<bool>,
    /// Name of the physical network adapter to bind the switch to. Cannot be used with switch_type or net_adapter_interface_description.
    #[serde(default, rename = "netAdapterName")]
    pub net_adapter_name: Option<String>,
    /// Interface description of the physical network adapter to bind the switch to. Cannot be used with switch_type or net_adapter_name.
    #[serde(default, rename = "netAdapterInterfaceDescription")]
    pub net_adapter_interface_description: Option<String>,
    /// Minimum bandwidth, in bits per second, allocated to the default flow when bandwidth mode is absolute.
    #[serde(default, rename = "defaultFlowMinimumBandwidthAbsolute")]
    pub default_flow_minimum_bandwidth_absolute: Option<i64>,
    /// Minimum bandwidth weight allocated to the default flow when bandwidth mode is weight.
    #[serde(default, rename = "defaultFlowMinimumBandwidthWeight")]
    pub default_flow_minimum_bandwidth_weight: Option<i64>,
    /// Enables Virtual Receive Side Scaling for the default queue.
    #[serde(default, rename = "defaultQueueVrssEnabled")]
    pub default_queue_vrss_enabled: Option<bool>,
    /// Enables Virtual Machine Multi-Queue for the default queue.
    #[serde(default, rename = "defaultQueueVmmqEnabled")]
    pub default_queue_vmmq_enabled: Option<bool>,
    /// Number of Virtual Machine Multi-Queue queue pairs for the default queue.
    #[serde(default, rename = "defaultQueueVmmqQueuePairs")]
    pub default_queue_vmmq_queue_pairs: Option<u32>,
    /// Notes to associate with the virtual switch.
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmSwitchInfo {
    pub name: String,
    pub id: String,
    pub switch_type: String,
    pub allow_management_os: bool,
    pub notes: String,
    pub net_adapter_interface_descriptions: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmSwitchOutput {
    pub switches: Vec<VmSwitchInfo>,
}

#[derive(Default)]
pub struct SetVmSwitchTool;

#[async_trait]
impl HyperVTool for SetVmSwitchTool {
    const NAME: &'static str = "hyperv_set_vm_switch";
    const DESCRIPTION: &'static str = "Configures a virtual switch.";
    type Input = SetVmSwitchInput;
    type Output = SetVmSwitchOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Switch name must not be empty".to_string(),
            ));
        }

        if input.switch_type.is_some()
            && (input.net_adapter_name.is_some()
                || input.net_adapter_interface_description.is_some())
        {
            return Err(ToolError::InvalidInput(
                "switch_type cannot be used with net_adapter_name or net_adapter_interface_description".to_string(),
            ));
        }

        if input.net_adapter_name.is_some() && input.net_adapter_interface_description.is_some() {
            return Err(ToolError::InvalidInput(
                "net_adapter_name and net_adapter_interface_description cannot both be provided"
                    .to_string(),
            ));
        }

        if input.switch_type.is_none()
            && input.allow_management_os.is_none()
            && input.net_adapter_name.is_none()
            && input.net_adapter_interface_description.is_none()
            && input.default_flow_minimum_bandwidth_absolute.is_none()
            && input.default_flow_minimum_bandwidth_weight.is_none()
            && input.default_queue_vrss_enabled.is_none()
            && input.default_queue_vmmq_enabled.is_none()
            && input.default_queue_vmmq_queue_pairs.is_none()
            && input.notes.is_none()
        {
            return Err(ToolError::InvalidInput(
                "At least one switch setting must be provided".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMSwitch -Name '{}'",
            escape_ps_string(&input.name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(switch_type) = &input.switch_type {
            args.push(format!("-SwitchType '{}'", escape_ps_string(switch_type)));
        }
        if let Some(allow) = input.allow_management_os {
            args.push(format!("-AllowManagementOS ${}", allow));
        }
        if let Some(adapter) = &input.net_adapter_name {
            args.push(format!("-NetAdapterName '{}'", escape_ps_string(adapter)));
        }
        if let Some(description) = &input.net_adapter_interface_description {
            args.push(format!(
                "-NetAdapterInterfaceDescription '{}'",
                escape_ps_string(description)
            ));
        }
        if let Some(bandwidth) = input.default_flow_minimum_bandwidth_absolute {
            args.push(format!(
                "-DefaultFlowMinimumBandwidthAbsolute {}",
                bandwidth
            ));
        }
        if let Some(weight) = input.default_flow_minimum_bandwidth_weight {
            args.push(format!("-DefaultFlowMinimumBandwidthWeight {}", weight));
        }
        if let Some(enabled) = input.default_queue_vrss_enabled {
            args.push(format!("-DefaultQueueVrssEnabled ${}", enabled));
        }
        if let Some(enabled) = input.default_queue_vmmq_enabled {
            args.push(format!("-DefaultQueueVmmqEnabled ${}", enabled));
        }
        if let Some(pairs) = input.default_queue_vmmq_queue_pairs {
            args.push(format!("-DefaultQueueVmmqQueuePairs {}", pairs));
        }
        if let Some(notes) = &input.notes {
            args.push(format!("-Notes '{}'", escape_ps_string(notes)));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object Name, Id, \
             @{{N='SwitchType';E={{$_.SwitchType.ToString()}}}}, \
             AllowManagementOS, Notes, NetAdapterInterfaceDescriptions | \
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

        let switches = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(switches.len());
        for switch in switches {
            let descriptions = match &switch["NetAdapterInterfaceDescriptions"] {
                serde_json::Value::Array(arr) => arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect(),
                _ => Vec::new(),
            };

            output.push(VmSwitchInfo {
                name: switch["Name"].as_str().unwrap_or_default().to_string(),
                id: switch["Id"].as_str().unwrap_or_default().to_string(),
                switch_type: switch["SwitchType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                allow_management_os: switch["AllowManagementOS"].as_bool().unwrap_or_default(),
                notes: switch["Notes"].as_str().unwrap_or_default().to_string(),
                net_adapter_interface_descriptions: descriptions,
            });
        }

        Ok(SetVmSwitchOutput { switches: output })
    }
}

register_tool!(SetVmSwitchTool);
