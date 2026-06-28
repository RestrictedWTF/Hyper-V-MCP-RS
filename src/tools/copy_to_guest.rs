use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CopyToGuestInput {
    pub vm_name: String,
    pub source_path: String,
    pub destination_path: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CopyToGuestOutput {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Default)]
pub struct CopyToGuestTool;

#[async_trait]
impl HyperVTool for CopyToGuestTool {
    const NAME: &'static str = "hyperv_copy_to_guest";
    const DESCRIPTION: &'static str =
        "Copies a file from the host filesystem into a guest VM using PowerShell Direct.";
    type Input = CopyToGuestInput;
    type Output = CopyToGuestOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("vm_name must not be empty".into()));
        }
        if input.source_path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "source_path must not be empty".into(),
            ));
        }
        if input.destination_path.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "destination_path must not be empty".into(),
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
$session = $null;
try {{
    $session = New-PSSession -VMName '{}' -Credential $cred -ErrorAction Stop;
    Copy-Item -Path '{}' -Destination '{}' -ToSession $session -Force -ErrorAction Stop;
    @{{ success = $true; error = $null }} | ConvertTo-Json -Compress
}} catch {{
    @{{ success = $false; error = $_.Exception.Message }} | ConvertTo-Json -Compress
}} finally {{
    if ($session) {{ Remove-PSSession $session -ErrorAction SilentlyContinue }}
}}
"#,
            escape_ps_string(&cred.password),
            escape_ps_string(&cred.username),
            escape_ps_string(&input.vm_name),
            escape_ps_string(&input.source_path),
            escape_ps_string(&input.destination_path),
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let output: CopyToGuestOutput = serde_json::from_str(&json)?;
        Ok(output)
    }
}

register_tool!(CopyToGuestTool);
