//! store 统计函数集成测试（成员D负责）
//!
//! 测试范围：
//!   - project_progress：完成/总数计数
//!   - project_cost：多成员不同费率成本核算
//!   - estimate_deviation：实际工时 vs 预估偏差
//!   - member_load：成员负载排行
//!   - overdue_tasks：逾期检测与边界
//!   - task_hours：任务累计工时
//!   - 持久化：save + load 往返

use rdplatform::auth;
use rdplatform::model::*;
use rdplatform::model::consts;
use rdplatform::store::Store;

// ==================== 辅助构造 ====================

/// 构造测试 Store：3 成员 + 2 项目 + 5 任务 + 工时记录
fn rich_store() -> Store {
    let mut s = Store::new();
    s.path = "data/test_db.json".into();

    // 成员
    let mk = |id: u32, username: &str, plain: &str, name: &str, role: Role, rate: f64| Member {
        id,
        username: username.into(),
        password_hash: auth::hash_password(plain),
        name: name.into(),
        role,
        rate,
    };
    s.members.push(mk(1, "pm1", "pm123", "王经理", Role::Pm, 300.0));
    s.members.push(mk(2, "dev1", "dv123", "李开发", Role::Dev, 200.0));
    s.members.push(mk(3, "qa1", "qa123", "赵测试", Role::Qa, 150.0));
    s.next_id = 4;

    // 项目
    s.projects.push(Project {
        id: 1,
        name: "项目A".into(),
        desc: String::new(),
        budget: 50000.0,
        start: "2026-09-01".into(),
        deadline: "2026-10-31".into(),
        status: consts::PRJ_ACTIVE.into(),
    });
    s.projects.push(Project {
        id: 2,
        name: "项目B".into(),
        desc: String::new(),
        budget: 30000.0,
        start: "2026-09-01".into(),
        deadline: "2026-09-20".into(),
        status: consts::PRJ_ACTIVE.into(),
    });

    // 任务：项目1 有 3 个任务（1 完成、1 进行、1 待办）；项目2 有 2 个任务（均待办）
    s.tasks.push(Task {
        id: 1, project_id: 1, title: "任务1-1".into(), assignee: Some(2),
        priority: consts::P_HIGH.into(), status: consts::T_DONE.into(),
        estimate_hours: 20.0, deadline: "2026-09-15".into(),
    });
    s.tasks.push(Task {
        id: 2, project_id: 1, title: "任务1-2".into(), assignee: Some(2),
        priority: consts::P_MEDIUM.into(), status: consts::T_DOING.into(),
        estimate_hours: 30.0, deadline: "2026-09-25".into(),
    });
    s.tasks.push(Task {
        id: 3, project_id: 1, title: "任务1-3".into(), assignee: Some(3),
        priority: consts::P_LOW.into(), status: consts::T_TODO.into(),
        estimate_hours: 10.0, deadline: "2026-10-10".into(),
    });
    s.tasks.push(Task {
        id: 4, project_id: 2, title: "任务2-1".into(), assignee: Some(2),
        priority: consts::P_HIGH.into(), status: consts::T_TODO.into(),
        estimate_hours: 15.0, deadline: "2026-09-18".into(),
    });
    s.tasks.push(Task {
        id: 5, project_id: 2, title: "任务2-2".into(), assignee: Some(3),
        priority: consts::P_MEDIUM.into(), status: consts::T_TODO.into(),
        estimate_hours: 8.0, deadline: "2026-09-19".into(),
    });

    // 工时：任务1 dev1 登 12h；任务2 dev1 登 25h；任务3 qa1 登 5h；任务4 dev1 登 6h
    let mk_ts = |id, task_id, member_id, hours| Timesheet {
        id, task_id, member_id, date: "2026-09-02".into(), hours, note: String::new(),
    };
    s.timesheets.push(mk_ts(101, 1, 2, 12.0));
    s.timesheets.push(mk_ts(102, 2, 2, 25.0));
    s.timesheets.push(mk_ts(103, 3, 3, 5.0));
    s.timesheets.push(mk_ts(104, 4, 2, 6.0));

    s
}

// ==================== project_progress ====================

#[test]
fn progress_project1() {
    let s = rich_store();
    let (done, total) = s.project_progress(1);
    assert_eq!(done, 1); // 只有任务1完成
    assert_eq!(total, 3);
}

#[test]
fn progress_project2() {
    let s = rich_store();
    let (done, total) = s.project_progress(2);
    assert_eq!(done, 0); // 都待办
    assert_eq!(total, 2);
}

#[test]
fn progress_nonexistent_project() {
    let s = rich_store();
    assert_eq!(s.project_progress(999), (0, 0));
}

// ==================== project_cost ====================

#[test]
fn cost_project1() {
    let s = rich_store();
    // 任务1(项目1): dev1(200/h) × 12h = 2400
    // 任务2(项目1): dev1(200/h) × 25h = 5000
    // 任务3(项目1): qa1(150/h) × 5h  = 750
    // 合计 = 8150
    assert_eq!(s.project_cost(1), 8150.0);
}

#[test]
fn cost_project2() {
    let s = rich_store();
    // 任务4(项目2): dev1(200/h) × 6h = 1200
    assert_eq!(s.project_cost(2), 1200.0);
}

#[test]
fn cost_nonexistent_project() {
    let s = rich_store();
    assert_eq!(s.project_cost(999), 0.0);
}

// ==================== estimate_deviation ====================

#[test]
fn deviation_project1() {
    let s = rich_store();
    // 预估: 20+30+10 = 60h
    // 实际: 12+25+5 = 42h
    // 偏差: 42-60 = -18（实际低于预估）
    assert_eq!(s.estimate_deviation(1), -18.0);
}

#[test]
fn deviation_project2() {
    let s = rich_store();
    // 预估: 15+8 = 23h
    // 实际: 6h
    // 偏差: 6-23 = -17
    assert_eq!(s.estimate_deviation(2), -17.0);
}

// ==================== member_load ====================

#[test]
fn member_load_totals() {
    let s = rich_store();
    let loads = s.member_load();
    // dev1: 12+25+6 = 43h
    // qa1:  5h
    // pm1 无工时
    assert_eq!(loads.len(), 2);
    assert!(loads.contains(&(2, "李开发".to_string(), 43.0)));
    assert!(loads.contains(&(3, "赵测试".to_string(), 5.0)));
}

#[test]
fn member_load_empty_store() {
    let s = Store::new();
    assert!(s.member_load().is_empty());
}

// ==================== overdue_tasks ====================

#[test]
fn overdue_with_rich_data() {
    let s = rich_store();
    // today=2026-09-20:
    //   任务1: 完成 → 不算
    //   任务2: deadline 09-25, 进行中 → 未逾期
    //   任务3: deadline 10-10, 待办 → 未逾期
    //   任务4: deadline 09-18, 待办 → 逾期
    //   任务5: deadline 09-19, 待办 → 逾期
    let overdue = s.overdue_tasks("2026-09-20");
    assert_eq!(overdue.len(), 2);
    assert!(overdue.iter().any(|t| t.id == 4));
    assert!(overdue.iter().any(|t| t.id == 5));
}

#[test]
fn overdue_none_when_all_before_deadline() {
    let s = rich_store();
    let overdue = s.overdue_tasks("2026-09-01");
    assert!(overdue.is_empty());
}

#[test]
fn overdue_excludes_completed() {
    let s = rich_store();
    // today=2026-09-16: 任务1 deadline 09-15 但已完成 → 不算逾期
    let overdue = s.overdue_tasks("2026-09-16");
    assert!(overdue.iter().all(|t| t.status != consts::T_DONE));
}

// ==================== task_hours ====================

#[test]
fn task_hours_accumulates() {
    let s = rich_store();
    // 任务2: dev1 登 25h
    assert_eq!(s.task_hours(2), 25.0);
    // 任务1: dev1 登 12h
    assert_eq!(s.task_hours(1), 12.0);
    // 无工时的任务
    assert_eq!(s.task_hours(5), 0.0);
}

#[test]
fn task_hours_nonexistent() {
    let s = rich_store();
    assert_eq!(s.task_hours(999), 0.0);
}

// ==================== 持久化往返 ====================

#[test]
fn save_and_load_roundtrip() {
    let mut s = rich_store();
    s.path = "data/test_roundtrip.json".into();
    s.save().unwrap();

    let loaded = Store::load("data/test_roundtrip.json").unwrap();
    assert_eq!(loaded.members.len(), 3);
    assert_eq!(loaded.projects.len(), 2);
    assert_eq!(loaded.tasks.len(), 5);
    assert_eq!(loaded.timesheets.len(), 4);

    // 统计函数在加载后仍然正确
    assert_eq!(loaded.project_progress(1), (1, 3));
    assert_eq!(loaded.project_cost(1), 8150.0);

    // 清理
    let _ = std::fs::remove_file("data/test_roundtrip.json");
}

// ==================== seed 数据验证 ====================

#[test]
fn seed_data_consistency() {
    let mut s = Store::new();
    s.seed();

    // 3 成员，1 项目，1 任务
    assert_eq!(s.members.len(), 3);
    assert_eq!(s.projects.len(), 1);
    assert_eq!(s.tasks.len(), 1);

    // 密码可校验
    assert!(auth::verify_password("pm123", &s.members[0].password_hash));
    assert!(auth::verify_password("dv123", &s.members[1].password_hash));
    assert!(auth::verify_password("qa123", &s.members[2].password_hash));

    // 角色
    assert!(s.members[0].role.is_pm());
    assert!(!s.members[1].role.is_pm());
}
