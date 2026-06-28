use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResetVmReplicationStatisticsInput {
    /// Name of the virtual machine whose replication statistics are to be reset.
pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmReplicationInfo {
    pub vm_name: String,
    pub state: String,
    pub health: String,
    pub mode: String,
    pub frequency_sec: i32,
    pub primary_server_name: String,
    pub replica_server_name: String,
    pub replica_server_port: i32,
    pub authentication_type: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ResetVmReplicationStatisticsOutput {
    pub replication: Vec<VmReplicationInfo>,
}

#[derive(Default)]
pub struct ResetVmReplicationStatisticsTool;

#[async_trait]
impl HyperVTool for ResetVmReplicationStatisticsTool {
    const NAME: &'static str = "hyperv_reset_vm_replication_statistics";
    const DESCRIPTION: &'static str = "Resets the replication statistics of a virtual machine.";
    type Input = ResetVmReplicationStatisticsInput;
    type Output = ResetVmReplicationStatisticsOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let name = input
            .name
            .ok_or_else(|| ToolError::InvalidInput("name is required".to_string()))?;
        if name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }

        let mut args = vec!["Reset-VMReplicationStatistics".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&name)));
        if let Some(computer) = &input.computer_name {
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }
        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object VMName, \
             @{{N='State';E={{$_.State.ToString()}}}}, \
             @{{N='Health';E={{$_.Health.ToString()}}}}, \
             @{{N='Mode';E={{$_.Mode.ToString()}}}}, \
             FrequencySec, PrimaryServerName, ReplicaServerName, ReplicaServerPort, \
             @{{N='AuthenticationType';E={{$_.AuthenticationType.ToString()}}}} | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let replication = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(replication.len());
        for item in replication {
            output.push(VmReplicationInfo {
                vm_name: item["VMName"].as_str().unwrap_or_default().to_string(),
                state: item["State"].as_str().unwrap_or_default().to_string(),
                health: item["Health"].as_str().unwrap_or_default().to_string(),
                mode: item["Mode"].as_str().unwrap_or_default().to_string(),
                frequency_sec: item["FrequencySec"].as_i64().unwrap_or_default() as i32,
                primary_server_name: item["PrimaryServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_name: item["ReplicaServerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                replica_server_port: item["ReplicaServerPort"]
                    .as_i64()
                    .unwrap_or_default() as i32,
                authentication_type: item["AuthenticationType"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            });
        }

        Ok(ResetVmReplicationStatisticsOutput {
            replication: output,
        })
    }
}

register_tool!(ResetVmReplicationStatisticsTool);
