use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use crate::sidecar::{SidecarClient, SidecarError};

#[derive(Clone)]
pub struct ToolContext {
    pub sidecar: Arc<SidecarClient>,
    pub timeout: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("PowerShell error: {message}")]
    PowerShell {
        message: String,
        category: String,
        fully_qualified_error_id: String,
    },
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Sidecar error: {0}")]
    Sidecar(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

impl ToolError {
    pub fn from_sidecar_err(err: &SidecarError) -> Self {
        Self::PowerShell {
            message: err.message.clone(),
            category: err.category.clone(),
            fully_qualified_error_id: err.fully_qualified_error_id.clone(),
        }
    }
}

#[async_trait]
pub trait HyperVTool: Default + Send + Sync + 'static {
    const NAME: &'static str;
    const DESCRIPTION: &'static str;
    type Input: DeserializeOwned + JsonSchema + Send;
    type Output: Serialize + JsonSchema + Send;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError>;
}

pub type ToolRunFn = for<'a> fn(
    &'a ToolContext,
    serde_json::Value,
) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ToolError>> + Send + 'a>>;

pub struct ToolMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: fn() -> serde_json::Value,
    pub output_schema: fn() -> serde_json::Value,
    pub run: ToolRunFn,
}

inventory::collect!(ToolMeta);

pub fn input_schema<T: HyperVTool>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T::Input)).unwrap()
}

pub fn output_schema<T: HyperVTool>() -> serde_json::Value {
    serde_json::to_value(schemars::schema_for!(T::Output)).unwrap()
}

pub fn run_tool<T: HyperVTool>(
    ctx: &ToolContext,
    input: serde_json::Value,
) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, ToolError>> + Send + '_>> {
    Box::pin(async move {
        let parsed = serde_json::from_value(input)?;
        let output = T::default().run(ctx, parsed).await?;
        Ok(serde_json::to_value(output)?)
    })
}

#[macro_export]
macro_rules! register_tool {
    ($tool:ty) => {
        inventory::submit! {
            $crate::tool::ToolMeta {
                name: <$tool as $crate::tool::HyperVTool>::NAME,
                description: <$tool as $crate::tool::HyperVTool>::DESCRIPTION,
                input_schema: $crate::tool::input_schema::<$tool>,
                output_schema: $crate::tool::output_schema::<$tool>,
                run: $crate::tool::run_tool::<$tool>,
            }
        }
    };
}
