use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmGroupMemberInfo {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmGroupInfo {
    pub name: String,
    pub id: String,
    #[serde(rename = "groupType")]
    pub group_type: String,
    #[serde(rename = "vmMembers")]
    pub vm_members: Vec<VmGroupMemberInfo>,
    #[serde(rename = "vmGroupMembers")]
    pub vm_group_members: Vec<VmGroupMemberInfo>,
    #[serde(rename = "computerName")]
    pub computer_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetVmGroupInput {
    /// Name of the virtual machine group.
    #[serde(default, rename = "name")]
    pub name: Option<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GetVmGroupOutput {
    pub groups: Vec<VmGroupInfo>,
}

#[derive(Default)]
pub struct GetVmGroupTool;

#[async_trait]
impl HyperVTool for GetVmGroupTool {
    const NAME: &'static str = "hyperv_get_vm_group";
    const DESCRIPTION: &'static str = "Gets virtual machine groups.";
    type Input = GetVmGroupInput;
    type Output = GetVmGroupOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Get-VMGroup".to_string()];
        if let Some(name) = &input.name {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!("-Name '{}'", escape_ps_string(name)));
        }
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "computer_name must not be empty when provided".to_string(),
                ));
            }
            args.push(format!(
                "-ComputerName '{}'",
                escape_ps_string(computer_name)
            ));
        }

        let ps = format!("{} | Select-Object Name, Id, @{{N='GroupType';E={{$_.GroupType.ToString()}}}}, VMMembers, VMGroupMembers, ComputerName | ConvertTo-Json -Compress -Depth 10", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let items = match raw {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        };

        let mut output = Vec::with_capacity(items.len());
        for item in items {
            output.push(parse_vm_group_info(&item));
        }

        Ok(GetVmGroupOutput { groups: output })
    }
}

fn parse_member_array(value: &serde_json::Value) -> Vec<VmGroupMemberInfo> {
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(|item| VmGroupMemberInfo {
                name: item["Name"].as_str().unwrap_or_default().to_string(),
                id: item["Id"].as_str().unwrap_or_default().to_string(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_vm_group_info(value: &serde_json::Value) -> VmGroupInfo {
    VmGroupInfo {
        name: value["Name"].as_str().unwrap_or_default().to_string(),
        id: value["Id"].as_str().unwrap_or_default().to_string(),
        group_type: value["GroupType"].as_str().unwrap_or_default().to_string(),
        vm_members: parse_member_array(&value["VMMembers"]),
        vm_group_members: parse_member_array(&value["VMGroupMembers"]),
        computer_name: value["ComputerName"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
    }
}

register_tool!(GetVmGroupTool);
