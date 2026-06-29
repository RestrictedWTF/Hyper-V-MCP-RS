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

#[tokio::test]
async fn sidecar_recovers_after_timeout() {
    let sidecar = SidecarClient::new().await.expect("spawn sidecar");

    // Run a command that outlasts a short timeout. The sidecar will still
    // emit a response for this request after we have given up, which would
    // leave stale output on stdout and desynchronize the pipe.
    let result = sidecar
        .execute(
            "Start-Sleep -Milliseconds 500",
            Duration::from_millis(50),
        )
        .await;
    assert!(result.is_err(), "expected timeout error");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("deadline has elapsed"),
        "expected deadline error, got: {}",
        err
    );

    // The next command must succeed because the client restarted the sidecar
    // and cleared any buffered/stale output.
    let output = sidecar
        .execute("'recovered' | ConvertTo-Json -Compress", Duration::from_secs(5))
        .await;
    assert!(output.is_ok(), "sidecar did not recover: {:?}", output);
    assert_eq!(output.unwrap(), "\"recovered\"");
}
