use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmHardDiskDriveInput {
    /// Name of the virtual machine whose hard disk drive is to be configured.
    pub vm_name: String,
    /// Hyper-V host on which the virtual machine resides. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// Type of the controller to which the hard disk drive is attached: IDE or SCSI.
    #[serde(default, rename = "controllerType")]
    pub controller_type: Option<String>,
    /// Number of the controller to which the hard disk drive is attached.
    #[serde(default, rename = "controllerNumber")]
    pub controller_number: Option<i32>,
    /// Location on the controller of the hard disk drive to configure.
    #[serde(default, rename = "controllerLocation")]
    pub controller_location: Option<i32>,
    /// Name of the hard disk drive to configure. Can be used instead of controller identification.
    #[serde(default)]
    pub name: Option<String>,
    /// New path to the virtual hard disk file.
    #[serde(default)]
    pub path: Option<String>,
    /// Name of the resource pool to which the virtual hard disk belongs.
    #[serde(default, rename = "poolName")]
    pub pool_name: Option<String>,
    /// Indicates whether the hard disk drive supports persistent reservations.
    #[serde(default, rename = "supportPersistentReservations")]
    pub support_persistent_reservations: Option<bool>,
    /// Maximum IOPS for the hard disk drive.
    #[serde(default, rename = "maximumIops")]
    pub maximum_iops: Option<u64>,
    /// Minimum IOPS for the hard disk drive.
    #[serde(default, rename = "minimumIops")]
    pub minimum_iops: Option<u64>,
    /// Quality of Service priority for storage I/O: VeryLow, Low, Normal, High, or VeryHigh.
    #[serde(default, rename = "qualityOfServicePriority")]
    pub quality_of_service_priority: Option<String>,
    /// Allows the cmdlet to use paths that have not been verified.
    #[serde(default, rename = "allowUnverifiedPaths")]
    pub allow_unverified_paths: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHardDiskDriveInfo {
    pub name: String,
    pub path: String,
    pub controller_type: String,
    pub controller_number: i32,
    pub controller_location: i32,
    pub disk_number: i32,
    pub pool_name: String,
    pub support_persistent_reservations: bool,
    pub maximum_iops: String,
    pub minimum_iops: String,
    pub quality_of_service_priority: String,
    pub computer_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmHardDiskDriveOutput {
    pub hard_disk_drives: Vec<VmHardDiskDriveInfo>,
}

#[derive(Default)]
pub struct SetVmHardDiskDriveTool;

#[async_trait]
impl HyperVTool for SetVmHardDiskDriveTool {
    const NAME: &'static str = "hyperv_set_vm_hard_disk_drive";
    const DESCRIPTION: &'static str = "Configures a virtual hard disk.";
    type Input = SetVmHardDiskDriveInput;
    type Output = SetVmHardDiskDriveOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMHardDiskDrive -VMName '{}'",
            escape_ps_string(&input.vm_name)
        )];

        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        if let Some(t) = &input.controller_type {
            args.push(format!("-ControllerType '{}'", escape_ps_string(t)));
        }
        if let Some(n) = input.controller_number {
            args.push(format!("-ControllerNumber {}", n));
        }
        if let Some(l) = input.controller_location {
            args.push(format!("-ControllerLocation {}", l));
        }
        if let Some(name) = &input.name {
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(path) = &input.path {
            args.push(format!("-Path '{}'", escape_ps_string(path)));
        }
        if let Some(pool) = &input.pool_name {
            args.push(format!("-PoolName '{}'", escape_ps_string(pool)));
        }
        if let Some(enabled) = input.support_persistent_reservations {
            args.push(format!("-SupportPersistentReservations ${}", enabled));
        }
        if let Some(max) = input.maximum_iops {
            args.push(format!("-MaximumIops {}", max));
        }
        if let Some(min) = input.minimum_iops {
            args.push(format!("-MinimumIops {}", min));
        }
        if let Some(priority) = &input.quality_of_service_priority {
            args.push(format!(
                "-QualityOfServicePriority '{}'",
                escape_ps_string(priority)
            ));
        }
        if input.allow_unverified_paths == Some(true) {
            args.push("-AllowUnverifiedPaths".to_string());
        }

        args.push("-PassThru".to_string());

        let ps = format!(
            "{} | Select-Object \
             Name, \
             Path, \
             @{{N='ControllerType';E={{$_.ControllerType.ToString()}}}}, \
             ControllerNumber, \
             ControllerLocation, \
             DiskNumber, \
             PoolName, \
             SupportPersistentReservations, \
             @{{N='MaximumIops';E={{$_.MaximumIops.ToString()}}}}, \
             @{{N='MinimumIops';E={{$_.MinimumIops.ToString()}}}}, \
             @{{N='QualityOfServicePriority';E={{$_.QualityOfServicePriority.ToString()}}}}, \
             ComputerName | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let drives = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(drives.len());
        for drive in drives {
            output.push(VmHardDiskDriveInfo {
                name: drive["Name"].as_str().unwrap_or_default().to_string(),
                path: drive["Path"].as_str().unwrap_or_default().to_string(),
                controller_type: drive["ControllerType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                controller_number: drive["ControllerNumber"].as_i64().unwrap_or_default() as i32,
                controller_location: drive["ControllerLocation"].as_i64().unwrap_or_default()
                    as i32,
                disk_number: drive["DiskNumber"].as_i64().unwrap_or_default() as i32,
                pool_name: drive["PoolName"].as_str().unwrap_or_default().to_string(),
                support_persistent_reservations: drive["SupportPersistentReservations"]
                    .as_bool()
                    .unwrap_or_default(),
                maximum_iops: drive["MaximumIops"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                minimum_iops: drive["MinimumIops"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                quality_of_service_priority: drive["QualityOfServicePriority"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: drive["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(SetVmHardDiskDriveOutput {
            hard_disk_drives: output,
        })
    }
}

register_tool!(SetVmHardDiskDriveTool);
