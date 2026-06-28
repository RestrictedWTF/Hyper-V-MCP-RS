use rmcp::model::{AnnotateAble, RawResource, Resource};

use crate::tool::ToolContext;

pub fn list_resources() -> Vec<Resource> {
    vec![
        RawResource::new("hyperv://vms/inventory", "VM Inventory")
            .with_description("List of all Hyper-V virtual machines")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResource::new("hyperv://networks/topology", "Network Topology")
            .with_description("Virtual switches and adapters")
            .with_mime_type("application/json")
            .no_annotation(),
        RawResource::new("hyperv://host/info", "Host Information")
            .with_description("Hyper-V host details")
            .with_mime_type("application/json")
            .no_annotation(),
    ]
}

pub async fn read_resource(ctx: &ToolContext, uri: &str) -> Result<String, String> {
    match uri {
        "hyperv://vms/inventory" => {
            let ps = "Get-VM | Select-Object Name, Id, @{N='State';E={$_.State.ToString()}} | ConvertTo-Json -Compress -Depth 3";
            ctx.sidecar
                .execute(ps, ctx.timeout)
                .await
                .map_err(|e| e.to_string())
        }
        "hyperv://networks/topology" => {
            let ps = "Get-VMSwitch | Select-Object Name, @{N='SwitchType';E={$_.SwitchType.ToString()}}, NetAdapterInterfaceDescription | ConvertTo-Json -Compress -Depth 3";
            ctx.sidecar
                .execute(ps, ctx.timeout)
                .await
                .map_err(|e| e.to_string())
        }
        "hyperv://host/info" => {
            let ps = "Get-VMHost | Select-Object ComputerName, LogicalProcessorCount, @{N='MemoryCapacity';E={$_.MemoryCapacity.ToString()}}, VirtualMachinePath | ConvertTo-Json -Compress -Depth 3";
            ctx.sidecar
                .execute(ps, ctx.timeout)
                .await
                .map_err(|e| e.to_string())
        }
        _ => Err(format!("unknown resource: {}", uri)),
    }
}
