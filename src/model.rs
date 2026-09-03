//! 数据模型（成员B负责，契约已定，字段勿改）
//!
//! 约定：
//!   - 日期一律使用字符串 "YYYY-MM-DD"（避免引入 chrono 依赖）；
//!   - 金额/工时一律 f64，费率单位：元/小时；
//!   - 优先级：高/中/低；任务状态：待办/进行/完成；项目状态：进行中/已完成；
//!   - 密码存储为 sha2 加盐哈希（见 auth.rs），本模型只保存哈希串。

use serde::{Deserialize, Serialize};

/// 成员角色（多学科团队视角：项目经理/开发/测试）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// 项目经理：建项目、派任务、定预算、看仪表盘
    Pm,
    /// 开发：领任务、更新状态、登记工时
    Dev,
    /// 测试：登记测试工时、跟踪任务质量
    Qa,
}

impl Role {
    /// 角色显示名
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Pm => "项目经理",
            Role::Dev => "开发",
            Role::Qa => "测试",
        }
    }

    /// 从显示名或标识解析（登录后由账号决定，一般用不上）
    pub fn from_str(s: &str) -> Option<Role> {
        match s {
            "Pm" | "pm" | "项目经理" => Some(Role::Pm),
            "Dev" | "dev" | "开发" => Some(Role::Dev),
            "Qa" | "qa" | "测试" => Some(Role::Qa),
            _ => None,
        }
    }

    /// 是否项目经理（权限判断）
    pub fn is_pm(&self) -> bool {
        matches!(self, Role::Pm)
    }
}

/// 成员（含登录账号与费率）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: u32,
    /// 登录用户名（唯一）
    pub username: String,
    /// 密码哈希（sha2 加盐，明文不落盘）
    pub password_hash: String,
    /// 显示姓名
    pub name: String,
    /// 角色
    pub role: Role,
    /// 费率（元/小时）——成本核算输入
    pub rate: f64,
}

/// 项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: u32,
    pub name: String,
    pub desc: String,
    /// 预算（元）
    pub budget: f64,
    /// 开始日期 YYYY-MM-DD
    pub start: String,
    /// 截止日期 YYYY-MM-DD
    pub deadline: String,
    /// 进行中 / 已完成
    pub status: String,
}

/// 任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u32,
    pub project_id: u32,
    pub title: String,
    /// 负责人成员 id（未指派为 None）
    pub assignee: Option<u32>,
    /// 高/中/低
    pub priority: String,
    /// 待办/进行/完成
    pub status: String,
    /// 预估工时（小时）
    pub estimate_hours: f64,
    /// 截止日期 YYYY-MM-DD
    pub deadline: String,
}

/// 工时记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timesheet {
    pub id: u32,
    pub task_id: u32,
    pub member_id: u32,
    /// 日期 YYYY-MM-DD
    pub date: String,
    /// 小时数
    pub hours: f64,
    pub note: String,
}

/// 任务/项目常量（避免字符串拼写错误）
pub mod consts {
    pub const P_HIGH: &str = "高";
    pub const P_MEDIUM: &str = "中";
    pub const P_LOW: &str = "低";

    pub const T_TODO: &str = "待办";
    pub const T_DOING: &str = "进行";
    pub const T_DONE: &str = "完成";

    pub const PRJ_ACTIVE: &str = "进行中";
    pub const PRJ_CLOSED: &str = "已完成";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_roundtrip() {
        assert_eq!(Role::Pm.as_str(), "项目经理");
        assert!(Role::Pm.is_pm());
        assert!(!Role::Dev.is_pm());
        assert_eq!(Role::from_str("dev"), Some(Role::Dev));
    }
}
