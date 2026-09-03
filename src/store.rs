//! 数据持久化与统计（成员B负责）
//!
//! 契约（勿改签名）：
//!   - Store 整体以 JSON 序列化到 data/db.json（path 字段不入盘）；
//!   - CRUD 与统计方法返回类型如下，页面层（C）与路由层（A）按此调用；
//!   - 已提供：load/save、seed（3 个演示账号）、today_iso（今日日期）。
//!   TODO(B)：补齐 CRUD 与 5 项统计函数的实现。

use crate::auth;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 默认数据文件
pub const DEFAULT_DATA_FILE: &str = "data/db.json";

/// 全量内存态数据（整体 JSON 落盘）
#[derive(Debug, Serialize, Deserialize)]
pub struct Store {
    pub members: Vec<Member>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub timesheets: Vec<Timesheet>,
    /// 自增 id 计数器
    pub next_id: u32,
    #[serde(skip)]
    pub path: String,
}

impl Store {
    /// 新建空 Store（绑定默认路径）
    pub fn new() -> Self {
        Store {
            members: Vec::new(),
            projects: Vec::new(),
            tasks: Vec::new(),
            timesheets: Vec::new(),
            next_id: 1,
            path: DEFAULT_DATA_FILE.to_string(),
        }
    }

    /// 从文件加载；文件不存在视为首次启动（返回空库）
    pub fn load(path: &str) -> Result<Store, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                let mut store: Store =
                    serde_json::from_str(&text).map_err(|e| format!("数据文件解析失败: {}", e))?;
                store.path = path.to_string();
                Ok(store)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut store = Store::new();
                store.path = path.to_string();
                Ok(store)
            }
            Err(e) => Err(format!("读取数据文件失败: {}", e)),
        }
    }

    /// 保存到文件（自动创建目录）
    pub fn save(&self) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("序列化失败: {}", e))?;
        if let Some(parent) = Path::new(&self.path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("创建数据目录失败: {}", e))?;
            }
        }
        std::fs::write(&self.path, json).map_err(|e| format!("写入数据文件失败: {}", e))
    }

    /// 申请一个新 id
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// 写入种子数据：3 个演示账号 + 一个演示项目/任务（登录演示用）
    ///
    /// 演示账号：pm1/pm123（项目经理）、dev1/dv123（开发）、qa1/qa123（测试）
    pub fn seed(&mut self) {
        if !self.members.is_empty() {
            return; // 已有数据不重复播种
        }
        let mk = |id: u32, username: &str, plain: &str, name: &str, role: Role, rate: f64| Member {
            id,
            username: username.to_string(),
            password_hash: auth::hash_password(plain),
            name: name.to_string(),
            role,
            rate,
        };
        self.members.push(mk(1, "pm1", "pm123", "王经理", Role::Pm, 300.0));
        self.members.push(mk(2, "dev1", "dv123", "李开发", Role::Dev, 200.0));
        self.members.push(mk(3, "qa1", "qa123", "赵测试", Role::Qa, 150.0));
        self.next_id = 4;

        self.projects.push(Project {
            id: 1,
            name: "智慧校园 App".to_string(),
            desc: "示例项目：校园信息门户与消息中心".to_string(),
            budget: 100000.0,
            start: "2026-09-01".to_string(),
            deadline: "2026-12-31".to_string(),
            status: consts::PRJ_ACTIVE.to_string(),
        });
        self.tasks.push(Task {
            id: 1,
            project_id: 1,
            title: "用户登录模块".to_string(),
            assignee: Some(2),
            priority: consts::P_HIGH.to_string(),
            status: consts::T_TODO.to_string(),
            estimate_hours: 40.0,
            deadline: "2026-09-15".to_string(),
        });
    }

    /// 今日日期 YYYY-MM-DD（UTC；演示够用）
    pub fn today_iso() -> String {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let days = (secs / 86400) as i64;
        // civil_from_days（Howard Hinnant 算法）
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = z - era * 146097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{:04}-{:02}-{:02}", y, m, d)
    }

    // ================= CRUD（TODO(B) 实现） =================

    pub fn add_member(&mut self, _m: Member) -> Result<(), String> {
        // TODO(B)
        Ok(())
    }
    pub fn find_member_by_username(&self, username: &str) -> Option<&Member> {
        // TODO(B)
        let _ = username;
        None
    }
    pub fn member_by_id(&self, id: u32) -> Option<&Member> {
        // TODO(B)
        let _ = id;
        None
    }
    pub fn add_project(&mut self, _p: Project) -> Result<(), String> {
        // TODO(B)
        Ok(())
    }
    pub fn project_by_id(&self, _id: u32) -> Option<&Project> {
        // TODO(B)
        None
    }
    pub fn add_task(&mut self, _t: Task) -> Result<(), String> {
        // TODO(B)
        Ok(())
    }
    pub fn task_by_id(&self, _id: u32) -> Option<&Task> {
        // TODO(B)
        None
    }
    pub fn set_task_status(&mut self, _task_id: u32, _status: &str) -> Result<(), String> {
        // TODO(B)
        Ok(())
    }
    pub fn add_timesheet(&mut self, _t: Timesheet) -> Result<(), String> {
        // TODO(B)
        Ok(())
    }
    /// 某任务累计实际工时
    pub fn task_hours(&self, _task_id: u32) -> f64 {
        // TODO(B)
        0.0
    }

    // ================= 统计（TODO(B) 实现，供仪表盘调用） =================

    /// 项目进度：(完成任务数, 总任务数)
    pub fn project_progress(&self, _project_id: u32) -> (u32, u32) {
        // TODO(B)
        (0, 0)
    }
    /// 项目实际成本 = Σ(工时 × 负责人费率)，超出预算用单独函数判断
    pub fn project_cost(&self, _project_id: u32) -> f64 {
        // TODO(B)
        0.0
    }
    /// 估算偏差（小时）= 实际总工时 - 预估总工时（正=超估）
    pub fn estimate_deviation(&self, _project_id: u32) -> f64 {
        // TODO(B)
        0.0
    }
    /// 成员负载：每人累计工时 Vec<(member_id, 姓名, 小时)>
    pub fn member_load(&self) -> Vec<(u32, String, f64)> {
        // TODO(B)
        Vec::new()
    }
    /// 逾期任务：未完成任务中截止日期 < today（today 由调用方传入）
    pub fn overdue_tasks(&self, _today: &str) -> Vec<Task> {
        // TODO(B)
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn today_iso_format() {
        let s = Store::today_iso();
        assert_eq!(s.len(), 10);
        assert!(s.contains('-'));
    }
}
