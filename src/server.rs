use std::sync::Arc;
use std::time::Duration;

use rmcp::{model::*, service::RequestContext, ErrorData as McpError, RoleServer, ServerHandler};
use tracing::error;

use crate::resources;
use crate::sidecar::SidecarClient;
use crate::tool::{ToolContext, ToolMeta};

#[derive(Clone)]
pub struct HypervServer {
    ctx: ToolContext,
}

impl HypervServer {
    pub async fn new() -> anyhow::Result<Self> {
        let sidecar = Arc::new(SidecarClient::new().await?);
        Ok(Self {
            ctx: ToolContext {
                sidecar,
                timeout: Duration::from_secs(30),
            },
        })
    }

    pub async fn sidecar_execute(
        &self,
        command: &str,
        timeout: Duration,
    ) -> anyhow::Result<String> {
        self.ctx.sidecar.execute(command, timeout).await
    }
}

impl ServerHandler for HypervServer {
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut tools = Vec::new();
        for meta in inventory::iter::<ToolMeta> {
            let schema_value = (meta.input_schema)();
            let schema_map = match schema_value {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };
            let output_schema_map = match (meta.output_schema)() {
                serde_json::Value::Object(map) => map,
                _ => serde_json::Map::new(),
            };

            tools.push(
                Tool::new(meta.name, meta.description, Arc::new(schema_map))
                    .with_raw_output_schema(Arc::new(output_schema_map)),
            );
        }
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let name = request.name.as_ref();
        for meta in inventory::iter::<ToolMeta> {
            if meta.name == name {
                let input = serde_json::Value::Object(request.arguments.unwrap_or_default());
                match (meta.run)(&self.ctx, input).await {
                    Ok(value) => {
                        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
                        return Ok(CallToolResult::success(vec![Content::text(text)]));
                    }
                    Err(e) => {
                        error!("tool {} failed: {}", name, e);
                        return Ok(CallToolResult::error(vec![Content::text(e.to_string())]));
                    }
                }
            }
        }
        Ok(CallToolResult::error(vec![Content::text(format!(
            "tool not found: {}",
            name
        ))]))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(
            resources::list_resources(),
        ))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        match resources::read_resource(&self.ctx, &request.uri).await {
            Ok(text) => Ok(ReadResourceResult::new(vec![ResourceContents::text(
                text,
                request.uri,
            )])),
            Err(e) => Err(McpError::internal_error(e, None)),
        }
    }
}
