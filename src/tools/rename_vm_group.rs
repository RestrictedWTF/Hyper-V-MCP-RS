use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmGroupRenameInfo {
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

#[derive(Debug, Serialize, JsonSchema)]
pub struct VmGroupMemberInfo {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RenameVmGroupInput {
    /// Current name of the virtual machine group.
pub name: String,
    /// New name for the group.
    #[serde(rename = "newName")]
    pub new_name: String,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct RenameVmGroupOutput {
    pub group: VmGroupRenameInfo,
}


#[derive(Default)]
pub struct RenameVmGroupTool;

#[async_trait]
impl HyperVTool for RenameVmGroupTool {
    const NAME: &'static str = "hyperv_rename_vm_group";
    const DESCRIPTION: &'static str = "Renames virtual machine groups.";
    type Input = RenameVmGroupInput;
    type Output = RenameVmGroupOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        let mut args = vec!["Rename-VMGroup".to_string()];
        if input.name.trim().is_empty() {
            return Err(ToolError::InvalidInput("name must not be empty".to_string()));
        }
        args.push(format!("-Name '{}'", escape_ps_string(&input.name)));
        if input.new_name.trim().is_empty() {
            return Err(ToolError::InvalidInput("new_name must not be empty".to_string()));
        }
        args.push(format!("-NewName '{}'", escape_ps_string(&input.new_name)));
        if let Some(computer_name) = &input.computer_name {
            if computer_name.trim().is_empty() {
                return Err(ToolError::InvalidInput("computer_name must not be empty when provided".to_string()));
            }
            args.push(format!("-ComputerName '{}'", escape_ps_string(computer_name)));
        }

        let ps = format!("{} | Select-Object Name, Id, @{{N='GroupType';E={{$_.GroupType.ToString()}}}}, VMMembers, VMGroupMembers, ComputerName | ConvertTo-Json -Compress -Depth 10", args.join(" "));

        let json = ctx
            .sidecar
            .execute(&ps, ctx.timeout)
            .await
            .map_err(|e| ToolError::Sidecar(e.to_string()))?;

        let json_sanitized = if json.trim().is_empty() { "[]" } else { &json };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let group = parse_vm_group_rename_info(&raw)?;
        Ok(RenameVmGroupOutput { group })

    }
}


fn parse_rename_member_array(value: &serde_json::Value) -> Vec<VmGroupMemberInfo> {
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

fn parse_vm_group_rename_info(raw: &serde_json::Value) -> Result<VmGroupRenameInfo, ToolError> {
    match raw {
        serde_json::Value::Array(arr) => arr
            .first()
            .ok_or_else(|| ToolError::InvalidInput("Cmdlet returned an empty array".to_string()))
            .map(build_rename_group_info),
        serde_json::Value::Object(_) => Ok(build_rename_group_info(raw)),
        other => Err(ToolError::InvalidInput(format!(
            "Unexpected cmdlet output: {}",
            other
        ))),
    }
}

fn build_rename_group_info(value: &serde_json::Value) -> VmGroupRenameInfo {
    VmGroupRenameInfo {
        name: value["Name"].as_str().unwrap_or_default().to_string(),
        id: value["Id"].as_str().unwrap_or_default().to_string(),
        group_type: value["GroupType"].as_str().unwrap_or_default().to_string(),
        vm_members: parse_rename_member_array(&value["VMMembers"]),
        vm_group_members: parse_rename_member_array(&value["VMGroupMembers"]),
        computer_name: value["ComputerName"].as_str().unwrap_or_default().to_string(),
    }
}

register_tool!(RenameVmGroupTool);
