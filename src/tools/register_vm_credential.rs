use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::EncryptedCredential;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RegisterVmCredentialInput {
    pub vm_name: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RegisterVmCredentialOutput {
    pub success: bool,
    pub message: String,
}

#[derive(Default)]
pub struct RegisterVmCredentialTool;

#[async_trait]
impl HyperVTool for RegisterVmCredentialTool {
    const NAME: &'static str = "hyperv_register_vm_credential";
    const DESCRIPTION: &'static str = "Registers a PowerShell Direct credential for a specific VM.";
    type Input = RegisterVmCredentialInput;
    type Output = RegisterVmCredentialOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".into()));
        }
        if input.username.trim().is_empty() {
            return Err(ToolError::InvalidInput("username must not be empty".into()));
        }
        if input.password.is_empty() {
            return Err(ToolError::InvalidInput("password must not be empty".into()));
        }

        let encrypted = EncryptedCredential::encrypt(&input.username, &input.password)
            .map_err(|e| ToolError::Sidecar(format!("failed to encrypt credential: {}", e)))?;

        {
            let mut store = ctx.config.credentials.write().map_err(|e| {
                ToolError::Sidecar(format!("failed to lock credential store: {}", e))
            })?;
            store.set(&input.vm_name, encrypted);
        }

        let store =
            ctx.config.credentials.read().map_err(|e| {
                ToolError::Sidecar(format!("failed to lock credential store: {}", e))
            })?;
        store
            .save()
            .map_err(|e| ToolError::Sidecar(format!("failed to save credentials: {}", e)))?;

        Ok(RegisterVmCredentialOutput {
            success: true,
            message: format!("Credential registered for '{}'.", input.vm_name),
        })
    }
}

register_tool!(RegisterVmCredentialTool);
