use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::error;

#[derive(Debug, Serialize)]
pub struct SidecarRequest {
    pub id: u64,
    pub command: String,
}

#[derive(Debug, Deserialize)]
pub struct SidecarResponse {
    pub id: u64,
    pub success: bool,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub error: Option<SidecarError>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SidecarError {
    #[serde(rename = "Message", default)]
    pub message: String,
    #[serde(rename = "Category", default)]
    pub category: String,
    #[serde(rename = "FullyQualifiedErrorId", default)]
    pub fully_qualified_error_id: String,
}

struct SidecarClientInner {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
}

pub struct SidecarClient {
    inner: Mutex<SidecarClientInner>,
}

impl SidecarClient {
    pub async fn new() -> anyhow::Result<Self> {
        let inner = Self::spawn_inner().await?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    async fn spawn_inner() -> anyhow::Result<SidecarClientInner> {
        let sidecar_script = include_str!("../sidecar/hyperv_sidecar.ps1");

        // PowerShell's -EncodedCommand expects the command as UTF-16 LE bytes,
        // base64-encoded. Using -EncodedCommand leaves stdin free for the
        // sidecar's JSON-RPC read loop.
        let utf16_bytes: Vec<u8> = sidecar_script
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let encoded = STANDARD.encode(&utf16_bytes);

        let mut child = Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-EncodedCommand",
                &encoded,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = child.stdout.take().expect("piped stdout");

        let reader = BufReader::new(stdout);

        Ok(SidecarClientInner {
            child,
            stdin,
            reader,
        })
    }

    pub async fn execute(
        &self,
        command: &str,
        timeout_duration: Duration,
    ) -> anyhow::Result<String> {
        let id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);
        let request = SidecarRequest {
            id,
            command: command.to_string(),
        };
        let request_json = serde_json::to_string(&request)?;

        let mut inner = self.inner.lock().await;

        match Self::execute_inner(&mut inner, id, &request_json, timeout_duration).await {
            Ok(result) => Ok(result),
            Err(e) if Self::is_dead(&mut inner) => {
                error!("sidecar process died, restarting");
                *inner = Self::spawn_inner().await?;
                Self::execute_inner(&mut inner, id, &request_json, timeout_duration).await
            }
            Err(e) => Err(e),
        }
    }

    async fn execute_inner(
        inner: &mut SidecarClientInner,
        id: u64,
        request_json: &str,
        timeout_duration: Duration,
    ) -> anyhow::Result<String> {
        inner.stdin.write_all(request_json.as_bytes()).await?;
        inner.stdin.write_all(b"\n").await?;
        inner.stdin.flush().await?;

        let response = timeout(timeout_duration, inner.read_response()).await??;

        if response.id != id {
            anyhow::bail!(
                "sidecar response id mismatch: expected {}, got {}",
                id,
                response.id
            );
        }

        if response.success {
            Ok(response.data.unwrap_or_default())
        } else {
            let err = response.error.unwrap_or(SidecarError {
                message: "unknown sidecar error".to_string(),
                category: "NotSpecified".to_string(),
                fully_qualified_error_id: "Unknown".to_string(),
            });
            anyhow::bail!(
                "PowerShell error: {} (category: {}, id: {})",
                err.message,
                err.category,
                err.fully_qualified_error_id
            )
        }
    }

    fn is_dead(inner: &mut SidecarClientInner) -> bool {
        matches!(inner.child.try_wait(), Ok(Some(_)))
    }
}

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

impl SidecarClientInner {
    async fn read_response(&mut self) -> anyhow::Result<SidecarResponse> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            anyhow::bail!("sidecar closed stdout");
        }
        let response: SidecarResponse = serde_json::from_str(&line)?;
        Ok(response)
    }
}
