//! 数据持久化与统计（成员B负责）
//!
//! 契约（勿改签名）：
//!   - Store 整体以 JSON 序列化到 data/db.json（path 字段不入盘）；
//!   - CRUD 与统计方法返回类型如下，页面层（C）与路由层（A）按此调用；
//!   - 已提供：load/save、seed（3 个演示账号）、today_iso（今日日期）；
//!   - CRUD/查询（add_*、find_*/by_id、set_task_status、task_hours）已实现；
//!   - 统计（project_progress / project_cost / estimate_deviation /
//!     member_load / overdue_tasks）已实现；
//!   契约已全部就绪：页面层（C）与路由层（A）可直接调用。

use crate::auth;
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

    // ================= CRUD / 查询（成员B：已实现） =================

    /// 新增成员。登录用户名唯一，重复时返回 Err 且不写入。
    pub fn add_member(&mut self, m: Member) -> Result<(), String> {
        if self.members.iter().any(|x| x.username == m.username) {
            return Err(format!("用户名已存在: {}", m.username));
        }
        self.members.push(m);
        Ok(())
    }

    /// 按登录用户名查找成员（登录流程依赖）
    pub fn find_member_by_username(&self, username: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.username == username)
    }

    /// 按 id 查找成员（会话鉴权依赖）
    pub fn member_by_id(&self, id: u32) -> Option<&Member> {
        self.members.iter().find(|m| m.id == id)
    }

    /// 新增项目。项目名（去除首尾空白后）为空时返回 Err。
    pub fn add_project(&mut self, p: Project) -> Result<(), String> {
        if p.name.trim().is_empty() {
            return Err("项目名不能为空".to_string());
        }
        self.projects.push(p);
        Ok(())
    }

    /// 按 id 查找项目
    pub fn project_by_id(&self, id: u32) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    /// 新建任务：必须挂到已存在的项目（project_id），且标题非空。
    pub fn add_task(&mut self, t: Task) -> Result<(), String> {
        if !self.projects.iter().any(|p| p.id == t.project_id) {
            return Err(format!("项目不存在: id={}", t.project_id));
        }
        if t.title.trim().is_empty() {
            return Err("任务标题不能为空".to_string());
        }
        self.tasks.push(t);
        Ok(())
    }

    /// 按 id 查找任务
    pub fn task_by_id(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// 任务状态流转。状态须为 待办/进行/完成（consts::T_*）之一。
    pub fn set_task_status(&mut self, task_id: u32, status: &str) -> Result<(), String> {
        if status != consts::T_TODO && status != consts::T_DOING && status != consts::T_DONE {
            return Err(format!("非法任务状态: {}", status));
        }
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| format!("任务不存在: id={}", task_id))?;
        task.status = status.to_string();
        Ok(())
    }

    /// 登记工时：任务须存在且 hours > 0。
    pub fn add_timesheet(&mut self, t: Timesheet) -> Result<(), String> {
        if !self.tasks.iter().any(|x| x.id == t.task_id) {
            return Err(format!("任务不存在: id={}", t.task_id));
        }
        if t.hours <= 0.0 {
            return Err(format!("工时必须大于 0: {}", t.hours));
        }
        self.timesheets.push(t);
        Ok(())
    }

    /// 某任务累计实际工时（工时登记超预估预警用；无记录返回 0）
    pub fn task_hours(&self, task_id: u32) -> f64 {
        self.timesheets
            .iter()
            .filter(|t| t.task_id == task_id)
            .map(|t| t.hours)
            .sum()
    }

    // ================= 统计（成员B：已实现，供仪表盘调用） =================

    /// 某项目全部任务 id 集合（统计函数内部复用）
    fn project_task_ids(&self, project_id: u32) -> HashSet<u32> {
        self.tasks
            .iter()
            .filter(|t| t.project_id == project_id)
            .map(|t| t.id)
            .collect()
    }

    /// 项目进度：(完成任务数, 总任务数)
    pub fn project_progress(&self, project_id: u32) -> (u32, u32) {
        let (done, total) = self
            .tasks
            .iter()
            .filter(|t| t.project_id == project_id)
            .fold((0u32, 0u32), |(done, total), t| {
                (done + u32::from(t.status == consts::T_DONE), total + 1)
            });
        (done, total)
    }

    /// 项目实际成本 = Σ(工时 × 登记人费率)，按成员当前 rate 计；
    /// 找不到登记人（数据异常）时该条按 0 计，不中断。
    pub fn project_cost(&self, project_id: u32) -> f64 {
        let task_ids = self.project_task_ids(project_id);
        self.timesheets
            .iter()
            .filter(|s| task_ids.contains(&s.task_id))
            .map(|s| {
                let rate = self.member_by_id(s.member_id).map_or(0.0, |m| m.rate);
                s.hours * rate
            })
            .sum()
    }

    /// 估算偏差（小时）= 实际总工时 - 预估总工时（正=超估，负=结余）
    pub fn estimate_deviation(&self, project_id: u32) -> f64 {
        let task_ids = self.project_task_ids(project_id);
        let estimate: f64 = self
            .tasks
            .iter()
            .filter(|t| t.project_id == project_id)
            .map(|t| t.estimate_hours)
            .sum();
        let actual: f64 = self
            .timesheets
            .iter()
            .filter(|s| task_ids.contains(&s.task_id))
            .map(|s| s.hours)
            .sum();
        actual - estimate
    }

    /// 成员负载：每人累计工时 Vec<(member_id, 姓名, 小时)>，全员返回（无工时者 0.0）
    pub fn member_load(&self) -> Vec<(u32, String, f64)> {
        self.members
            .iter()
            .map(|m| {
                let hours: f64 = self
                    .timesheets
                    .iter()
                    .filter(|s| s.member_id == m.id)
                    .map(|s| s.hours)
                    .sum();
                (m.id, m.name.clone(), hours)
            })
            .collect()
    }

    /// 逾期任务：未完成（待办/进行）且截止日期 < today（today 由调用方传入，
    /// 定长 YYYY-MM-DD 可直接字典序比较；无截止日期的不计逾期）
    pub fn overdue_tasks(&self, today: &str) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|t| t.status != consts::T_DONE && !t.deadline.is_empty() && t.deadline.as_str() < today)
            .cloned()
            .collect()
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

    // ---- CRUD/查询自测（成员B数据测试） ----

    fn mk_member(id: u32, username: &str, name: &str, role: Role) -> Member {
        Member {
            id,
            username: username.to_string(),
            password_hash: "哈希占位".to_string(),
            name: name.to_string(),
            role,
            rate: 100.0,
        }
    }

    #[test]
    fn member_add_find_and_duplicate_rejected() {
        let mut s = Store::new();
        assert!(s.add_member(mk_member(1, "u1", "甲", Role::Dev)).is_ok());
        assert_eq!(s.members.len(), 1);

        // 命中查询
        assert_eq!(s.find_member_by_username("u1").map(|m| m.id), Some(1));
        assert_eq!(s.member_by_id(1).map(|m| m.name.as_str()), Some("甲"));
        // 未命中
        assert!(s.find_member_by_username("nobody").is_none());
        assert!(s.member_by_id(9).is_none());

        // 用户名重复：返回 Err 且不新增
        assert!(s.add_member(mk_member(2, "u1", "乙", Role::Qa)).is_err());
        assert_eq!(s.members.len(), 1);
    }

    #[test]
    fn project_add_find_and_blank_name_rejected() {
        let mut s = Store::new();
        let p = Project {
            id: 7,
            name: "新项目".to_string(),
            desc: String::new(),
            budget: 5000.0,
            start: "2026-10-01".to_string(),
            deadline: "2026-12-01".to_string(),
            status: consts::PRJ_ACTIVE.to_string(),
        };
        assert!(s.add_project(p).is_ok());
        assert_eq!(s.project_by_id(7).map(|p| p.name.as_str()), Some("新项目"));
        assert!(s.project_by_id(1).is_none());

        // 空白项目名被拒
        let blank = Project {
            id: 8,
            name: "   ".to_string(),
            desc: String::new(),
            budget: 0.0,
            start: String::new(),
            deadline: String::new(),
            status: consts::PRJ_ACTIVE.to_string(),
        };
        assert!(s.add_project(blank).is_err());
        assert_eq!(s.projects.len(), 1);
    }

    #[test]
    fn task_add_status_flow_and_orphan_rejected() {
        let mut s = Store::new();
        s.seed(); // seed 直写：项目 1、任务 1

        // 任务必须挂到已存在项目
        let orphan = Task {
            id: 9,
            project_id: 999,
            title: "孤儿任务".to_string(),
            assignee: None,
            priority: consts::P_MEDIUM.to_string(),
            status: consts::T_TODO.to_string(),
            estimate_hours: 8.0,
            deadline: "2026-09-20".to_string(),
        };
        assert!(s.add_task(orphan).is_err());
        assert_eq!(s.tasks.len(), 1);

        let t = Task {
            id: 9,
            project_id: 1,
            title: "报表导出".to_string(),
            assignee: Some(2),
            priority: consts::P_HIGH.to_string(),
            status: consts::T_TODO.to_string(),
            estimate_hours: 16.0,
            deadline: "2026-09-20".to_string(),
        };
        assert!(s.add_task(t).is_ok());
        assert_eq!(s.task_by_id(9).map(|t| t.title.as_str()), Some("报表导出"));
        assert!(s.task_by_id(1).is_some()); // seed 任务可查
        assert!(s.task_by_id(2).is_none());

        // 状态流转：待办 → 进行 → 完成
        assert!(s.set_task_status(9, consts::T_DOING).is_ok());
        assert_eq!(s.task_by_id(9).unwrap().status, consts::T_DOING);
        assert!(s.set_task_status(9, consts::T_DONE).is_ok());
        assert_eq!(s.task_by_id(9).unwrap().status, consts::T_DONE);
        // 非法状态 / 任务不存在 → Err
        assert!(s.set_task_status(9, "已完成").is_err());
        assert!(s.set_task_status(999, consts::T_DONE).is_err());
    }

    #[test]
    fn timesheet_add_and_task_hours() {
        let mut s = Store::new();
        s.seed(); // 任务 1 存在
        assert_eq!(s.task_hours(1), 0.0);

        let mk = |id: u32, task: u32, member: u32, hours: f64| Timesheet {
            id,
            task_id: task,
            member_id: member,
            date: "2026-09-02".to_string(),
            hours,
            note: String::new(),
        };
        assert!(s.add_timesheet(mk(10, 1, 2, 4.0)).is_ok());
        assert!(s.add_timesheet(mk(11, 1, 2, 6.0)).is_ok());
        assert!(s.add_timesheet(mk(12, 1, 3, 2.5)).is_ok());
        assert_eq!(s.task_hours(1), 12.5);
        assert_eq!(s.task_hours(999), 0.0); // 无记录任务

        // 无效登记被拒：任务不存在 / 工时非正
        assert!(s.add_timesheet(mk(13, 999, 2, 1.0)).is_err());
        assert!(s.add_timesheet(mk(14, 1, 2, 0.0)).is_err());
        assert_eq!(s.timesheets.len(), 3);
    }

    // ---- 5 项统计自测（成员B数据测试） ----

    fn sheet(id: u32, task: u32, member: u32, hours: f64) -> Timesheet {
        Timesheet {
            id,
            task_id: task,
            member_id: member,
            date: "2026-09-02".to_string(),
            hours,
            note: String::new(),
        }
    }

    #[test]
    fn stats_progress_cost_deviation() {
        let mut s = Store::new();
        s.seed(); // 项目1；任务1（李开发 rate200、预估40h、待办）；成员 pm1/dev1/qa1

        // 同项目补任务2（qa1、完成、预估10h）、任务3（未指派、待办、预估5h）
        let t2 = Task {
            id: 2,
            project_id: 1,
            title: "任务2".to_string(),
            assignee: Some(3),
            priority: consts::P_MEDIUM.to_string(),
            status: consts::T_DONE.to_string(),
            estimate_hours: 10.0,
            deadline: "2026-09-10".to_string(),
        };
        let t3 = Task {
            id: 3,
            project_id: 1,
            title: "任务3".to_string(),
            assignee: None,
            priority: consts::P_LOW.to_string(),
            status: consts::T_TODO.to_string(),
            estimate_hours: 5.0,
            deadline: "2026-09-20".to_string(),
        };
        assert!(s.add_task(t2).is_ok());
        assert!(s.add_task(t3).is_ok());

        // 进度：3 个任务、1 个完成；无此项目 → (0,0)
        assert_eq!(s.project_progress(1), (1, 3));
        assert_eq!(s.project_progress(99), (0, 0));

        // 成本：任务1→dev 4h×200=800；任务2→qa 2h×150=300
        assert!(s.add_timesheet(sheet(10, 1, 2, 4.0)).is_ok());
        assert!(s.add_timesheet(sheet(11, 2, 3, 2.0)).is_ok());
        assert_eq!(s.project_cost(1), 1100.0);
        assert_eq!(s.project_cost(99), 0.0);

        // 偏差：实际 6h − 预估(40+10+5=55)h = −49（未超估）
        assert_eq!(s.estimate_deviation(1), -49.0);
        assert_eq!(s.estimate_deviation(99), 0.0);
    }

    #[test]
    fn stats_member_load_all_members() {
        let mut s = Store::new();
        s.seed(); // pm1/dev1/qa1；任务1 存在
        assert!(s.add_timesheet(sheet(1, 1, 2, 4.0)).is_ok());
        assert!(s.add_timesheet(sheet(2, 1, 2, 1.0)).is_ok());
        assert!(s.add_timesheet(sheet(3, 1, 3, 2.0)).is_ok());

        // 全员返回：dev 5h、qa 2h、pm 0h
        assert_eq!(
            s.member_load(),
            vec![
                (1, "王经理".to_string(), 0.0),
                (2, "李开发".to_string(), 5.0),
                (3, "赵测试".to_string(), 2.0),
            ]
        );
    }

    #[test]
    fn stats_overdue_tasks_unfinished_past_deadline() {
        let mut s = Store::new();
        s.seed(); // 任务1：待办，截止 2026-09-15（未逾期）
        let mk_task = |id: u32, title: &str, status: &str, deadline: &str| Task {
            id,
            project_id: 1,
            title: title.to_string(),
            assignee: Some(2),
            priority: consts::P_MEDIUM.to_string(),
            status: status.to_string(),
            estimate_hours: 8.0,
            deadline: deadline.to_string(),
        };
        // 逾期：待办已过截止
        let late = mk_task(2, "已逾期", consts::T_TODO, "2026-09-01");
        // 进行中但已过截止 → 也算逾期
        let late_doing = mk_task(3, "进行中逾期", consts::T_DOING, "2026-09-05");
        // 已完成但已过截止 → 不算逾期
        let done_late = mk_task(4, "完成但晚", consts::T_DONE, "2026-09-01");
        // 无截止日期 → 不算逾期
        let no_deadline = mk_task(5, "无期限", consts::T_TODO, "");
        for t in [late, late_doing, done_late, no_deadline] {
            assert!(s.add_task(t).is_ok());
        }

        let overdue = s.overdue_tasks("2026-09-08");
        let titles: Vec<&str> = overdue.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(overdue.len(), 2);
        assert!(titles.contains(&"已逾期"));
        assert!(titles.contains(&"进行中逾期"));
        assert!(!titles.contains(&"用户登录模块")); // 未过截止
        assert!(!titles.contains(&"完成但晚"));
        assert!(!titles.contains(&"无期限"));
        // 更早的 today：全部未逾期
        assert!(s.overdue_tasks("2026-01-01").is_empty());
    }

    // ---- seed 复核（成员B收尾） ----

    #[test]
    fn seed_content_and_idempotent() {
        let mut s = Store::new();
        s.seed();

        // 3 个演示账号：账号/姓名/角色/费率 逐一核对，密码哈希可验
        assert_eq!(s.members.len(), 3);
        let pm = s.member_by_id(1).expect("pm1 应存在");
        assert_eq!(
            (pm.username.as_str(), pm.name.as_str(), pm.rate),
            ("pm1", "王经理", 300.0)
        );
        assert_eq!(pm.role, Role::Pm);
        assert!(crate::auth::verify_password("pm123", &pm.password_hash));
        assert!(!pm.password_hash.contains("pm123"), "哈希中不得含明文");

        let dev = s.member_by_id(2).expect("dev1 应存在");
        assert_eq!(
            (dev.username.as_str(), dev.name.as_str(), dev.rate),
            ("dev1", "李开发", 200.0)
        );
        assert_eq!(dev.role, Role::Dev);
        assert!(crate::auth::verify_password("dv123", &dev.password_hash));

        let qa = s.member_by_id(3).expect("qa1 应存在");
        assert_eq!(
            (qa.username.as_str(), qa.name.as_str(), qa.rate),
            ("qa1", "赵测试", 150.0)
        );
        assert_eq!(qa.role, Role::Qa);
        assert!(crate::auth::verify_password("qa123", &qa.password_hash));
        // 错误密码必然验不过
        assert!(!crate::auth::verify_password("wrong", &pm.password_hash));

        // 自增 id 与集合规模
        assert_eq!(s.next_id, 4);
        assert_eq!(s.projects.len(), 1);
        assert_eq!(s.tasks.len(), 1);
        assert!(s.timesheets.is_empty());

        // 示例项目字段
        let p = s.project_by_id(1).expect("项目1 应存在");
        assert_eq!(p.name, "智慧校园 App");
        assert_eq!(p.budget, 100000.0);
        assert_eq!((p.start.as_str(), p.deadline.as_str()), ("2026-09-01", "2026-12-31"));
        assert_eq!(p.status, consts::PRJ_ACTIVE);

        // 示例任务：字段 + 引用一致性（项目1、指派 dev1=id2 均存在）
        let t = s.task_by_id(1).expect("任务1 应存在");
        assert_eq!(t.title, "用户登录模块");
        assert_eq!(t.project_id, 1);
        assert_eq!(t.assignee, Some(2));
        assert_eq!(t.priority, consts::P_HIGH);
        assert_eq!(t.status, consts::T_TODO);
        assert_eq!(t.estimate_hours, 40.0);
        assert_eq!(t.deadline, "2026-09-15");
        assert!(s.project_by_id(t.project_id).is_some());
        assert!(s.member_by_id(t.assignee.unwrap()).is_some());

        // 幂等：重复播种不叠加、不覆盖已有数据
        s.seed();
        assert_eq!(s.members.len(), 3);
        assert_eq!(s.projects.len(), 1);
        assert_eq!(s.next_id, 4);
    }

    #[test]
    fn seed_persist_and_reload_roundtrip() {
        // 模拟 main.rs 首启流程：空库 → seed → save 到文件
        let path = "target/seed_review_tmp.json";
        let _ = std::fs::remove_file(path);
        let mut s = Store::new();
        s.path = path.to_string();
        s.seed();
        assert!(s.save().is_ok());

        // 模拟重启：load → 再 seed（不得重复播种），内容完整
        let loaded = Store::load(path).expect("种子文件应可加载");
        assert_eq!(loaded.members.len(), 3);
        assert_eq!(loaded.next_id, 4);
        let mut loaded = loaded;
        loaded.seed();
        assert_eq!(loaded.members.len(), 3);
        assert_eq!(loaded.projects.len(), 1);
        assert_eq!(loaded.tasks.len(), 1);
        assert!(crate::auth::verify_password(
            "pm123",
            &loaded.member_by_id(1).unwrap().password_hash
        ));
        assert_eq!(loaded.project_by_id(1).unwrap().budget, 100000.0);
        assert_eq!(loaded.task_by_id(1).unwrap().title, "用户登录模块");

        let _ = std::fs::remove_file(path);
    }
}
