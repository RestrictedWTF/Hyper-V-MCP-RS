use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ps_escape::escape_ps_string;
use crate::register_tool;
use crate::tool::{HyperVTool, ToolContext, ToolError};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddVmGroupMemberInput {
    /// Name of the virtual machine group to which members are added.
    #[serde(rename = "groupName")]
    pub group_name: String,
    /// Names of virtual machines to add to the group.
    #[serde(default, rename = "vmNames")]
    pub vm_names: Vec<String>,
    /// Names of virtual machine groups to add as nested members.
    #[serde(default, rename = "vmGroupNames")]
    pub vm_group_names: Vec<String>,
    /// Hyper-V host to target. Defaults to localhost.
    #[serde(default, rename = "computerName")]
    pub computer_name: Option<String>,
}

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

#[derive(Debug, Serialize, JsonSchema)]
pub struct AddVmGroupMemberOutput {
    /// The virtual machine group after the members were added.
    pub group: VmGroupInfo,
}

#[derive(Default)]
pub struct AddVmGroupMemberTool;

#[async_trait]
impl HyperVTool for AddVmGroupMemberTool {
    const NAME: &'static str = "hyperv_add_vm_group_member";
    const DESCRIPTION: &'static str = "Adds group members to a virtual machine group.";
    type Input = AddVmGroupMemberInput;
    type Output = AddVmGroupMemberOutput;

    async fn run(&self, ctx: &ToolContext, input: Self::Input) -> Result<Self::Output, ToolError> {
        if input.group_name.trim().is_empty() {
            return Err(ToolError::InvalidInput(
                "groupName must not be empty".to_string(),
            ));
        }

        if input.vm_names.is_empty() && input.vm_group_names.is_empty() {
            return Err(ToolError::InvalidInput(
                "At least one of vmNames or vmGroupNames must be provided".to_string(),
            ));
        }

        for name in &input.vm_names {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vmNames must not contain empty entries".to_string(),
                ));
            }
        }

        for name in &input.vm_group_names {
            if name.trim().is_empty() {
                return Err(ToolError::InvalidInput(
                    "vmGroupNames must not contain empty entries".to_string(),
                ));
            }
        }

        let mut last_json = String::new();

        if !input.vm_names.is_empty() {
            let vm_exprs: Vec<String> = input
                .vm_names
                .iter()
                .map(|n| format!("(Get-VM -Name '{}')", escape_ps_string(n)))
                .collect();

            let mut args = vec![format!(
                "Add-VMGroupMember -Name '{}'",
                escape_ps_string(&input.group_name)
            )];
            args.push(format!("-VM {}", vm_exprs.join(",")));

            if let Some(computer) = &input.computer_name {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }

            let ps = format!(
                "{} | Select-Object Name, Id, \
                 @{{N='GroupType';E={{$_.GroupType.ToString()}}}}, \
                 VMMembers, VMGroupMembers, ComputerName | \
                 ConvertTo-Json -Compress -Depth 10",
                args.join(" ")
            );

            last_json = ctx
                .sidecar
                .execute(&ps, ctx.timeout)
                .await
                .map_err(|e| ToolError::Sidecar(e.to_string()))?;
        }

        if !input.vm_group_names.is_empty() {
            let group_exprs: Vec<String> = input
                .vm_group_names
                .iter()
                .map(|n| format!("(Get-VMGroup -Name '{}')", escape_ps_string(n)))
                .collect();

            let mut args = vec![format!(
                "Add-VMGroupMember -Name '{}'",
                escape_ps_string(&input.group_name)
            )];
            args.push(format!("-VMGroupMember {}", group_exprs.join(",")));

            if let Some(computer) = &input.computer_name {
                args.push(format!("-ComputerName '{}'", escape_ps_string(computer)));
            }

            let ps = format!(
                "{} | Select-Object Name, Id, \
                 @{{N='GroupType';E={{$_.GroupType.ToString()}}}}, \
                 VMMembers, VMGroupMembers, ComputerName | \
                 ConvertTo-Json -Compress -Depth 10",
                args.join(" ")
            );

            last_json = ctx
                .sidecar
                .execute(&ps, ctx.timeout)
                .await
                .map_err(|e| ToolError::Sidecar(e.to_string()))?;
        }

        let json_sanitized = if last_json.trim().is_empty() {
            "[]"
        } else {
            &last_json
        };
        let raw: serde_json::Value = serde_json::from_str(json_sanitized)?;

        let group = parse_vm_group(&raw)?;
        Ok(AddVmGroupMemberOutput { group })
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

fn parse_vm_group(raw: &serde_json::Value) -> Result<VmGroupInfo, ToolError> {
    match raw {
        serde_json::Value::Array(arr) => arr
            .first()
            .ok_or_else(|| ToolError::InvalidInput("Cmdlet returned an empty array".to_string()))
            .map(build_vm_group_info),
        serde_json::Value::Object(_) => Ok(build_vm_group_info(raw)),
        other => Err(ToolError::InvalidInput(format!(
            "Unexpected cmdlet output: {}",
            other
        ))),
    }
}

fn build_vm_group_info(value: &serde_json::Value) -> VmGroupInfo {
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

register_tool!(AddVmGroupMemberTool);
