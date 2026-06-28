use std::time::Duration;

use rmcp::transport::stdio;
use rmcp::ServiceExt;
use tracing::{error, info};

use hyperv_mcp::server::HypervServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("starting hyperv-mcp");

    let server = HypervServer::new().await?;

    // Elevation check
    let elevation_check = "([Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) | ConvertTo-Json -Compress";
    let result = server
        .sidecar_execute(elevation_check, Duration::from_secs(5))
        .await;

    match result {
        Ok(json) => {
            let is_admin: bool = serde_json::from_str(&json).unwrap_or(false);
            if !is_admin {
                eprintln!("Error: Hyper-V MCP server must be run with Administrative privileges.");
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("elevation check failed: {}", e);
            eprintln!("Error: Hyper-V MCP server must be run with Administrative privileges.");
            std::process::exit(1);
        }
    }

    info!("elevation check passed, serving stdio");
    server.serve(stdio()).await?.waiting().await?;
    Ok(())
}
