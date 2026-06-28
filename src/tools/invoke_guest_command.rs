use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InvokeGuestCommandInput {
    pub vm_name: String,
    pub script_block: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct InvokeGuestCommandOutput {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct InvokeGuestCommandTool;

#[async_trait]
impl HyperVTool for InvokeGuestCommandTool {
    const NAME: &'static str = "hyperv_invoke_guest_command";
    const DESCRIPTION: &'static str =
        "Runs a PowerShell script block inside a guest VM using PowerShell Direct.";
    type Input = InvokeGuestCommandInput;
    type Output = InvokeGuestCommandOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".into()));
        }
        if input.script_block.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "script_block must not be empty".into(),
            ));
        }

        let cred = ctx
            .config
            .resolve(&input.vm_name, input.username.as_deref(), input.password.as_deref())
            .ok_or_else(|| {
                ToolError::InvalidInput(
                    "No credential found. Provide username/password explicitly or register a credential with hyperv_register_vm_credential.".into(),
                )
            })?;

        let ps = format!(
            r#"
$secure = ConvertTo-SecureString '{}' -AsPlainText -Force;
$cred = New-Object System.Management.Automation.PSCredential('{}', $secure);
$sb = [scriptblock]::Create('{}');
try {{
    $result = Invoke-Command -VMName '{}' -Credential $cred -ScriptBlock $sb -ErrorAction Stop | Out-String;
    @{{ success = $true; output = $result; error = $null }} | ConvertTo-Json -Compress
}} catch {{
    @{{ success = $false; output = ''; error = $_.Exception.Message }} | ConvertTo-Json -Compress
}}
"#,
            escape_ps_string(&cred.password),
            escape_ps_string(&cred.username),
            escape_ps_string(&input.script_block),
            escape_ps_string(&input.vm_name),
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let output: InvokeGuestCommandOutput = serde_json::from_str(&json)?;
        Ok(output)
    }
}

register_tool!(InvokeGuestCommandTool);
