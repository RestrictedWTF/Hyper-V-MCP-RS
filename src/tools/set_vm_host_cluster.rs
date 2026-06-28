use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetVmHostClusterInput {
    /// Name of the virtual machine host cluster to configure.
    #[serde(rename = "clusterName")]
    pub cluster_name: String,
    /// Location of the shared storage for the virtual machine host cluster.
    #[serde(default, rename = "sharedStoragePath")]
    pub shared_storage_path: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmHostClusterConfig {
    pub cluster_name: String,
    pub shared_storage_path: String,
    pub path: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SetVmHostClusterOutput {
    pub clusters: Vec<VmHostClusterConfig>,
}

#[derive(Default)]
pub struct SetVmHostClusterTool;

#[async_trait]
impl HyperVTool for SetVmHostClusterTool {
    const NAME: &'static str = "hyperv_set_vm_host_cluster";
    const DESCRIPTION: &'static str = "Configures a virtual machine host cluster.";
    type Input = SetVmHostClusterInput;
    type Output = SetVmHostClusterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.cluster_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Cluster name must not be empty".to_string(),
            ));
        }

        let mut args = vec![format!(
            "Set-VMHostCluster -ClusterName '{}'",
            escape_ps_string(&input.cluster_name)
        )];

        if let Some(path) = &input.shared_storage_path {
            args.push(format!("-SharedStoragePath '{}'", escape_ps_string(path)));
        }


        let ps = format!(
            "{} | Select-Object ClusterName, SharedStoragePath, Path | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let clusters = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(clusters.len());
        for cluster in clusters {
            output.push(VmHostClusterConfig {
                cluster_name: cluster["ClusterName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                shared_storage_path: cluster["SharedStoragePath"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                path: cluster["Path"].as_str().unwrap_or_default().to_string(),
            });
        }

        Ok(SetVmHostClusterOutput { clusters: output })
    }
}

register_tool!(SetVmHostClusterTool);
