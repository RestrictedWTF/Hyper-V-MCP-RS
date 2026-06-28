use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmHostClusterInput {
    /// Name of the virtual machine host cluster. If omitted, the cmdlet uses the
    /// default behaviour of the underlying PowerShell command.
    #[serde(default)]
    pub cluster_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostClusterInfo {
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    #[serde(rename = "sharedStoragePath")]
    pub shared_storage_path: String,
    #[serde(rename = "computerName")]
    pub computer_name: String,
    #[serde(rename = "isDeleted")]
    pub is_deleted: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmHostClusterOutput {
    pub clusters: Vec<VmHostClusterInfo>,
}

#[derive(Default)]
pub struct GetVmHostClusterTool;

#[async_trait]
impl HyperVTool for GetVmHostClusterTool {
    const NAME: &'static str = "hyperv_get_vm_host_cluster";
    const DESCRIPTION: &'static str = "Gets virtual machine host clusters.";
    type Input = GetVmHostClusterInput;
    type Output = GetVmHostClusterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMHostCluster".to_string()];
        if let Some(cluster_name) = &input.cluster_name {
            args.push(format!("-ClusterName '{}'", escape_ps_string(cluster_name)));
        }

        let ps = format!(
            "{} | Select-Object ClusterName, SharedStoragePath, ComputerName, IsDeleted | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: Value = serde_json::from_str(json_sanitized)?;

        let clusters = match raw {
            Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(clusters.len());
        for cluster in clusters {
            output.push(VmHostClusterInfo {
                cluster_name: cluster["ClusterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                shared_storage_path: cluster["SharedStoragePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                computer_name: cluster["ComputerName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                is_deleted: cluster["IsDeleted"].as_bool().unwrap_or_default(),
            });
        }

        Ok(GetVmHostClusterOutput { clusters: output })
    }
}

register_tool!(GetVmHostClusterTool);
