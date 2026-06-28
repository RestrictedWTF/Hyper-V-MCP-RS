use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmVideoInput {
    /// Name of the virtual machine whose video settings will be configured.
    pub vm_name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Resolution type for the virtual machine display: Default or Single.
    #[serde(default, rename = "resolutionType")]
    pub resolution_type: Option<String>,
    /// Horizontal resolution, in pixels.
    #[serde(default, rename = "horizontalResolution")]
    pub horizontal_resolution: Option<u32>,
    /// Vertical resolution, in pixels.
    #[serde(default, rename = "verticalResolution")]
    pub vertical_resolution: Option<u32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmVideoInfo {
    pub vm_name: String,
    pub computer_name: String,
    pub resolution_type: String,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmVideoOutput {
    pub videos: Vec<VmVideoInfo>,
}

#[derive(Default)]
pub struct SetVmVideoTool;

#[async_trait]
impl HyperVTool for SetVmVideoTool {
    const NAME: &'static str = "hyperv_set_vm_video";
    const DESCRIPTION: &'static str = "Configures video settings for virtual machines.";
    type Input = SetVmVideoInput;
    type Output = SetVmVideoOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMVideo -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(resolution_type) = &input.resolution_type {
            args.push(format!(
                "-ResolutionType '{}'",
                escape_ps_string(resolution_type)
            ));
        }
        if let Some(horizontal) = input.horizontal_resolution {
            args.push(format!("-HorizontalResolution {}", horizontal));
        }
        if let Some(vertical) = input.vertical_resolution {
            args.push(format!("-VerticalResolution {}", vertical));
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             @{{N='VMName';E={{$_.VMName}}}}, \
             @{{N='ComputerName';E={{$_.ComputerName}}}}, \
             @{{N='ResolutionType';E={{$_.ResolutionType.ToString()}}}}, \
             HorizontalResolution, VerticalResolution | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let videos = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(videos.len());
        for video in videos {
            output.push(VmVideoInfo {
                vm_name: video["VMName"].as_str().unwrap_or_default().to_string(),
                computer_name: video["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                resolution_type: video["ResolutionType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                horizontal_resolution: video["HorizontalResolution"].as_u64().unwrap_or_default()
                    as u32,
                vertical_resolution: video["VerticalResolution"].as_u64().unwrap_or_default()
                    as u32,
            });
        }

        Ok(SetVmVideoOutput { videos: output })
    }
}

register_tool!(SetVmVideoTool);
