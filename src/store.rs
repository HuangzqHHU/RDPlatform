//! 数据持久化与统计（成员B负责）
//!
//! 契约（勿改签名，以组长确认版为准）：
//!   - Store 整体以 JSON 序列化到 data/db.json（path 字段不入盘）；
//!   - 成本口径：实际成本 = Σ(工时小时数 × 登记工时成员的小时费率)；
//!   - 进度口径：完成任务数 / 项目总任务数；
//!   - 估算偏差 = 项目任务 Σ实际工时 - Σ预估工时（正数表示超估）；
//!   - 逾期 = 状态非"完成" 且 deadline < today（YYYY-MM-DD 字符串比较即时间序）；
//!   - 另提供 member_tasks / member_timesheets 供"我的视图"（C）调用；
//!   - 测试：9 个基线（组长提供）+ 2 个 seed 复核（成员B补充）。

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
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("序列化失败: {}", e))?;
        if let Some(parent) = Path::new(&self.path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| format!("创建数据目录失败: {}", e))?;
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
        self.members
            .push(mk(1, "pm1", "pm123", "王经理", Role::Pm, 300.0));
        self.members
            .push(mk(2, "dev1", "dv123", "李开发", Role::Dev, 200.0));
        self.members
            .push(mk(3, "qa1", "qa123", "赵测试", Role::Qa, 150.0));
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

    // ================= 成员 CRUD =================

    /// 新增成员（校验用户名非空且不重复）
    pub fn add_member(&mut self, m: Member) -> Result<(), String> {
        if m.username.is_empty() {
            return Err("用户名不能为空".to_string());
        }
        if self.members.iter().any(|x| x.username == m.username) {
            return Err(format!("用户名 {} 已存在", m.username));
        }
        self.members.push(m);
        Ok(())
    }

    /// 按用户名查找成员（登录用）
    pub fn find_member_by_username(&self, username: &str) -> Option<&Member> {
        self.members.iter().find(|m| m.username == username)
    }

    /// 按 id 查找成员
    pub fn member_by_id(&self, id: u32) -> Option<&Member> {
        self.members.iter().find(|m| m.id == id)
    }

    // ================= 项目 CRUD =================

    /// 新增项目（校验名称非空）
    pub fn add_project(&mut self, p: Project) -> Result<(), String> {
        if p.name.is_empty() {
            return Err("项目名称不能为空".to_string());
        }
        self.projects.push(p);
        Ok(())
    }

    /// 按 id 查找项目
    pub fn project_by_id(&self, id: u32) -> Option<&Project> {
        self.projects.iter().find(|p| p.id == id)
    }

    // ================= 任务 CRUD =================

    /// 新增任务（校验标题非空）
    pub fn add_task(&mut self, t: Task) -> Result<(), String> {
        if t.title.is_empty() {
            return Err("任务标题不能为空".to_string());
        }
        self.tasks.push(t);
        Ok(())
    }

    /// 按 id 查找任务
    pub fn task_by_id(&self, id: u32) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    /// 修改任务状态（待办/进行/完成）
    pub fn set_task_status(&mut self, task_id: u32, status: &str) -> Result<(), String> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or("任务不存在")?;
        task.status = status.to_string();
        Ok(())
    }

    // ================= 工时 CRUD =================

    /// 新增工时记录（校验任务存在、小时数为正）
    pub fn add_timesheet(&mut self, t: Timesheet) -> Result<(), String> {
        if self.task_by_id(t.task_id).is_none() {
            return Err("任务不存在".to_string());
        }
        if t.hours <= 0.0 {
            return Err("工时必须大于 0".to_string());
        }
        self.timesheets.push(t);
        Ok(())
    }

    /// 某任务累计实际工时
    pub fn task_hours(&self, task_id: u32) -> f64 {
        self.timesheets
            .iter()
            .filter(|t| t.task_id == task_id)
            .map(|t| t.hours)
            .sum()
    }

    // ================= 统计（仪表盘用） =================

    /// 某成员所有工时记录（按成员查，供"我的视图"用）
    pub fn member_timesheets(&self, member_id: u32) -> Vec<&Timesheet> {
        self.timesheets.iter().filter(|t| t.member_id == member_id).collect()
    }

    /// 某成员负责的任务（供"我的视图"用）
    pub fn member_tasks(&self, member_id: u32) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| t.assignee == Some(member_id))
            .collect()
    }

    /// 项目进度：(完成任务数, 总任务数)
    pub fn project_progress(&self, project_id: u32) -> (u32, u32) {
        let tasks: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| t.project_id == project_id)
            .collect();
        let total = tasks.len() as u32;
        let done = tasks.iter().filter(|t| t.status == consts::T_DONE).count() as u32;
        (done, total)
    }

    /// 项目实际成本 = Σ(工时 × 登记工时成员的费率)
    pub fn project_cost(&self, project_id: u32) -> f64 {
        self.timesheets
            .iter()
            .filter(|ts| {
                self.task_by_id(ts.task_id)
                    .map(|t| t.project_id == project_id)
                    .unwrap_or(false)
            })
            .map(|ts| {
                let rate = self.member_by_id(ts.member_id).map(|m| m.rate).unwrap_or(0.0);
                ts.hours * rate
            })
            .sum()
    }

    /// 估算偏差（小时）= 项目任务 Σ实际工时 - Σ预估工时（正数表示实际超预估）
    pub fn estimate_deviation(&self, project_id: u32) -> f64 {
        let tasks: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| t.project_id == project_id)
            .collect();
        let estimate: f64 = tasks.iter().map(|t| t.estimate_hours).sum();
        let actual: f64 = tasks.iter().map(|t| self.task_hours(t.id)).sum();
        actual - estimate
    }

    /// 成员负载：每人累计工时 Vec<(member_id, 姓名, 小时)>
    pub fn member_load(&self) -> Vec<(u32, String, f64)> {
        let mut loads: Vec<(u32, f64)> = Vec::new();
        for ts in &self.timesheets {
            match loads.iter_mut().find(|(id, _)| *id == ts.member_id) {
                Some((_, h)) => *h += ts.hours,
                None => loads.push((ts.member_id, ts.hours)),
            }
        }
        loads
            .into_iter()
            .map(|(id, hours)| {
                let name = self.member_by_id(id).map(|m| m.name.clone()).unwrap_or_default();
                (id, name, hours)
            })
            .collect()
    }

    /// 逾期任务：状态非"完成" 且 deadline < today
    pub fn overdue_tasks(&self, today: &str) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|t| t.status != consts::T_DONE && t.deadline.as_str() < today)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded() -> Store {
        let mut s = Store::new();
        s.seed();
        s
    }

    /// 测试用工时记录构造器（date/note 固定）
    fn ts(id: u32, task_id: u32, member_id: u32, hours: f64) -> Timesheet {
        Timesheet {
            id,
            task_id,
            member_id,
            date: "2026-09-02".to_string(),
            hours,
            note: String::new(),
        }
    }

    #[test]
    fn today_iso_format() {
        let s = Store::today_iso();
        assert_eq!(s.len(), 10);
        assert!(s.contains('-'));
    }

    #[test]
    fn seed_creates_accounts() {
        let s = seeded();
        assert_eq!(s.members.len(), 3);
        assert!(s.find_member_by_username("pm1").is_some());
        assert!(s.find_member_by_username("nobody").is_none());
    }

    #[test]
    fn member_crud_unique_username() {
        let mut s = seeded();
        // 重复用户名被拒绝
        let dup = s.members[0].clone();
        assert!(s.add_member(dup).is_err());
        // 空用户名被拒绝
        let bad = Member {
            id: 99,
            username: String::new(),
            password_hash: String::new(),
            name: "x".into(),
            role: Role::Dev,
            rate: 1.0,
        };
        assert!(s.add_member(bad).is_err());
    }

    #[test]
    fn task_status_flow() {
        let mut s = seeded();
        s.set_task_status(1, consts::T_DONE).unwrap();
        assert_eq!(s.task_by_id(1).unwrap().status, consts::T_DONE);
        assert!(s.set_task_status(999, consts::T_DONE).is_err());
    }

    #[test]
    fn timesheet_and_task_hours() {
        let mut s = seeded();
        let id1 = s.alloc_id();
        s.add_timesheet(ts(id1, 1, 2, 8.0)).unwrap();
        let id2 = s.alloc_id();
        s.add_timesheet(ts(id2, 1, 2, 2.0)).unwrap();
        assert_eq!(s.task_hours(1), 10.0);
        // 非法工时
        let id3 = s.alloc_id();
        assert!(s.add_timesheet(ts(id3, 1, 2, -1.0)).is_err());
        // 任务不存在
        let id4 = s.alloc_id();
        assert!(s.add_timesheet(ts(id4, 999, 2, 1.0)).is_err());
    }

    #[test]
    fn project_cost_uses_member_rate() {
        let mut s = seeded();
        // dev1(200元/h) 登记 10h，qa1(150元/h) 登记 4h → 成本 = 2000+600=2600
        let id1 = s.alloc_id();
        s.add_timesheet(ts(id1, 1, 2, 10.0)).unwrap();
        let id2 = s.alloc_id();
        s.add_timesheet(ts(id2, 1, 3, 4.0)).unwrap();
        assert_eq!(s.project_cost(1), 2600.0);
    }

    #[test]
    fn progress_and_deviation() {
        let mut s = seeded();
        assert_eq!(s.project_progress(1), (0, 1));
        s.set_task_status(1, consts::T_DONE).unwrap();
        assert_eq!(s.project_progress(1), (1, 1));
        // 预估 40h，实际 0 → 偏差 -40
        assert_eq!(s.estimate_deviation(1), -40.0);
    }

    #[test]
    fn overdue_detection() {
        let mut s = seeded();
        // seed 任务 deadline 2026-09-15；today 设 2026-09-20 → 逾期；today 2026-09-01 → 未逾期
        assert_eq!(s.overdue_tasks("2026-09-20").len(), 1);
        assert!(s.overdue_tasks("2026-09-01").is_empty());
        // 完成任务不算逾期
        s.set_task_status(1, consts::T_DONE).unwrap();
        assert!(s.overdue_tasks("2026-09-20").is_empty());
    }

    #[test]
    fn member_load_and_views() {
        let mut s = seeded();
        let id1 = s.alloc_id();
        s.add_timesheet(ts(id1, 1, 2, 8.0)).unwrap();
        let id2 = s.alloc_id();
        s.add_timesheet(ts(id2, 1, 3, 3.0)).unwrap();
        let loads = s.member_load();
        assert_eq!(loads.len(), 2);
        assert!(loads.contains(&(2, "李开发".to_string(), 8.0)));
        // 我的视图
        assert_eq!(s.member_tasks(2).len(), 1);
        assert_eq!(s.member_timesheets(3).len(), 1);
    }

    // ---- seed 复核（成员B补充） ----

    #[test]
    fn seed_content_and_idempotent() {
        let mut s = Store::new();
        s.seed();

        // 3 个演示账号：字段逐一核对，密码哈希可验且不含明文
        assert_eq!(s.members.len(), 3);
        let pm = s.member_by_id(1).unwrap();
        assert_eq!(
            (pm.username.as_str(), pm.name.as_str(), pm.rate),
            ("pm1", "王经理", 300.0)
        );
        assert_eq!(pm.role, Role::Pm);
        assert!(crate::auth::verify_password("pm123", &pm.password_hash));
        assert!(!pm.password_hash.contains("pm123"));
        let dev = s.member_by_id(2).unwrap();
        assert_eq!(
            (dev.username.as_str(), dev.name.as_str(), dev.rate),
            ("dev1", "李开发", 200.0)
        );
        assert!(crate::auth::verify_password("dv123", &dev.password_hash));
        let qa = s.member_by_id(3).unwrap();
        assert_eq!(
            (qa.username.as_str(), qa.name.as_str(), qa.rate),
            ("qa1", "赵测试", 150.0)
        );
        assert!(crate::auth::verify_password("qa123", &qa.password_hash));
        assert!(!crate::auth::verify_password("wrong", &pm.password_hash));

        // 自增 id 与示例项目/任务字段、引用一致性
        assert_eq!(s.next_id, 4);
        assert!(s.timesheets.is_empty());
        let p = s.project_by_id(1).unwrap();
        assert_eq!(p.name, "智慧校园 App");
        assert_eq!(p.budget, 100000.0);
        assert_eq!((p.start.as_str(), p.deadline.as_str()), ("2026-09-01", "2026-12-31"));
        assert_eq!(p.status, consts::PRJ_ACTIVE);
        let t = s.task_by_id(1).unwrap();
        assert_eq!(t.title, "用户登录模块");
        assert_eq!((t.project_id, t.assignee), (1, Some(2)));
        assert_eq!(t.priority, consts::P_HIGH);
        assert_eq!(t.status, consts::T_TODO);
        assert_eq!(t.estimate_hours, 40.0);
        assert_eq!(t.deadline, "2026-09-15");
        assert!(s.project_by_id(t.project_id).is_some());
        assert!(s.member_by_id(t.assignee.unwrap()).is_some());

        // 幂等：重复播种不叠加
        s.seed();
        assert_eq!(s.members.len(), 3);
        assert_eq!(s.next_id, 4);
    }

    #[test]
    fn seed_persist_and_reload_roundtrip() {
        // 模拟 main.rs 首启流程：空库 → seed → save；重启 load → 再 seed 不重复
        let path = "target/seed_review_tmp.json";
        let _ = std::fs::remove_file(path);
        let mut s = Store::new();
        s.path = path.to_string();
        s.seed();
        assert!(s.save().is_ok());

        let loaded = Store::load(path).unwrap();
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
