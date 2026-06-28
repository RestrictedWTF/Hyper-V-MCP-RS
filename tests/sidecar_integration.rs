use std::time::Duration;

use hyperv_mcp::sidecar::SidecarClient;

#[tokio::test]
async fn sidecar_spawns_and_responds() {
    let sidecar = SidecarClient::new().await.expect("spawn sidecar");
    let output = sidecar
        .execute(
            "'hello' | ConvertTo-Json -Compress",
            Duration::from_secs(15),
        )
        .await;
    assert!(output.is_ok(), "sidecar command failed: {:?}", output);
    assert_eq!(output.unwrap(), "\"hello\"");
}

#[tokio::test]
async fn sidecar_returns_error_for_invalid_command() {
    let sidecar = SidecarClient::new().await.expect("spawn sidecar");
    let output = sidecar
        .execute(
            "Get-Command DefinitelyMissing-Cmdlet",
            Duration::from_secs(5),
        )
        .await;
    assert!(output.is_err(), "expected an error for missing cmdlet");
}
