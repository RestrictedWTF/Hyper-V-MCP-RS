use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestVmNetworkAdapterInput {
    /// Name of the virtual machine whose network adapter is to be tested.
    #[serde(rename = "vmName")]
    pub vm_name: String,
    /// Name of the virtual network adapter to test. If omitted, the cmdlet tests the default adapter.
    #[serde(default)]
    pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
    /// IP address of the sender virtual machine.
    #[serde(rename = "senderIPAddress")]
    pub sender_ip_address: String,
    /// IP address of the receiver virtual machine.
    #[serde(rename = "receiverIPAddress")]
    pub receiver_ip_address: String,
    /// Sequence number to use to generate ICMP Ping packets.
    #[serde(rename = "sequenceNumber")]
    pub sequence_number: i32,
    /// Target the sender virtual machine. At least one of sender or receiver must be true.
    #[serde(default)]
    pub sender: bool,
    /// Target the receiver virtual machine. At least one of sender or receiver must be true.
    #[serde(default)]
    pub receiver: bool,
    /// MAC address for the next hop VM required for non-Hyper-V Network Virtualization configurations.
    #[serde(default, rename = "nextHopMacAddress")]
    pub next_hop_mac_address: Option<String>,
    /// ID of a virtual subnet.
    #[serde(default, rename = "isolationId")]
    pub isolation_id: Option<i32>,
    /// Payload size of the ICMP Ping packets.
    #[serde(default, rename = "payloadSize")]
    pub payload_size: Option<i32>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct TestVmNetworkAdapterOutput {
    /// Round-trip time, in milliseconds, measured by the connectivity test.
    #[serde(rename = "roundTripTime")]
    pub round_trip_time: i32,
}

#[derive(Default)]
pub struct TestVmNetworkAdapterTool;

#[async_trait]
impl HyperVTool for TestVmNetworkAdapterTool {
    const NAME: &'static str = "hyperv_test_vm_network_adapter";
    const DESCRIPTION: &'static str = "Tests connectivity between virtual machines.";
    type Input = TestVmNetworkAdapterInput;
    type Output = TestVmNetworkAdapterOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.vm_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "VM name must not be empty".to_string(),
            ));
        }
        if input.sender_ip_address.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Sender IP address must not be empty".to_string(),
            ));
        }
        if input.receiver_ip_address.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "Receiver IP address must not be empty".to_string(),
            ));
        }
        if !input.sender && !input.receiver {
            return Err(ToolError::InvalidInput(
                "At least one of sender or receiver must be true".to_string(),
            ));
        }

        let mut args = vec!["Test-VMNetworkAdapter".to_string()];
        args.push(format!("-VMName '{}'", escape_ps_string(&input.vm_name)));

        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Adapter name must not be empty".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer) = &input.computer_name {
            if computer.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Computer name must not be empty".to_string(),
                ));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
        }

        args.push(format!(
            "-SenderIPAddress '{}'",
            escape_ps_string(&input.sender_ip_address)
        ));
        args.push(format!(
            "-ReceiverIPAddress '{}'",
            escape_ps_string(&input.receiver_ip_address)
        ));
        args.push(format!("-SequenceNumber {}", input.sequence_number));

        if input.sender {
            args.push("-Sender".to_string());
        }
        if input.receiver {
            args.push("-Receiver".to_string());
        }
        if let Some(mac) = &input.next_hop_mac_address {
            if mac.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "Next hop MAC address must not be empty".to_string(),
                ));
            }
            args.push(format!("-NextHopMacAddress '{}'", escape_ps_string(mac)));
        }
        if let Some(id) = input.isolation_id {
            args.push(format!("-IsolationId {}", id));
        }
        if let Some(size) = input.payload_size {
            args.push(format!("-PayloadSize {}", size));
        }

        args.push("-Passthru".to_string());

        let ps = format!(
            "{} | Select-Object RoundTripTime | ConvertTo-Json -Compress -Depth 3",
            args.join(" ")
        );

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let result = match raw {
            serde_json::Value::Array(arr) if arr.len() == 1 => arr[0].clone(),
            serde_json::Value::Array(arr) if arr.is_empty() => {
                return Err(ToolError::Sidecar(
                    "empty response from sidecar".to_string(),
                ));
            }
            serde_json::Value::Array(arr) => {
                return Err(ToolError::Sidecar(format!(
                    "unexpected multiple results from Test-VMNetworkAdapter: {}",
                    arr.len()
                )));
            }
            other => other,
        };

        let round_trip_time = result["RoundTripTime"].as_i64().ok_or_else(|| {
            ToolError::Sidecar(format!(
                "unexpected RoundTripTime value: {}",
                result["RoundTripTime"]
            ))
        })? as i32;

        Ok(TestVmNetworkAdapterOutput { round_trip_time })
    }
}

register_tool!(TestVmNetworkAdapterTool);
