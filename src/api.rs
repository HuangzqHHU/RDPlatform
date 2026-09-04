//! 路由分发与业务处理（成员A负责，骨架+登录完整）
//!
//! 流程：解析请求 → 登录相关直接处理；其余先鉴权（Cookie token → 会话 → 成员）
//!      → 按角色分发页面/动作 → 写操作后 store.save()。
//!
//! 依赖契约：
//!   - store.find_member_by_username / member_by_id（B 实现，登录依赖）
//!   - store 各 CRUD 与统计（B）；page 页面函数（C）；auth（D 增强）

use crate::auth;
use crate::http::{self, HttpRequest};
use crate::model::{Member, Project, Role, Task, Timesheet, consts};
use crate::page;
use crate::store::Store;

/// 路由总入口：返回完整 HTTP 响应
pub fn dispatch(req: &HttpRequest, store: &mut Store) -> String {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/login") => handle_login_page(req),
        ("POST", "/login") => handle_login_post(req, store),
        ("GET", "/logout") => handle_logout(req),
        _ => handle_authed(req, store),
    }
}

// ============ 登录 / 登出 ============

fn handle_login_page(req: &HttpRequest) -> String {
    if let Some(member) = current_member(req, &Store::new()) {
        // 已登录直接进首页（此分支基本不会走到，仅防御）
        let _ = member;
        http::redirect("/")
    } else {
        http::http_response(200, "text/html; charset=utf-8", &page::login_page(None))
    }
}

fn handle_login_post(req: &HttpRequest, store: &mut Store) -> String {
    let username = req.param("username").unwrap_or_default();
    let password = req.param("password").unwrap_or_default();
    let result = store
        .find_member_by_username(&username)
        .map(|m| (m.id, m.password_hash.clone()))
        .filter(|(_, hash)| auth::verify_password(&password, hash));
    match result {
        Some((member_id, _)) => {
            let token = auth::create_session(member_id);
            http::redirect_with_cookie("/", &format!("token={}", token))
        }
        None => http::http_response(
            200,
            "text/html; charset=utf-8",
            &page::login_page(Some("用户名或密码错误")),
        ),
    }
}

fn handle_logout(req: &HttpRequest) -> String {
    if let Some(token) = auth::token_from_cookie(&req.cookie) {
        auth::destroy_session(&token);
    }
    http::redirect("/login")
}

// ============ 鉴权 ============

/// 解析当前登录成员（Cookie → 会话 → member）
pub fn current_member<'a>(req: &HttpRequest, store: &'a Store) -> Option<&'a Member> {
    let token = auth::token_from_cookie(&req.cookie)?;
    let member_id = auth::session_member_id(&token)?;
    store.member_by_id(member_id)
}

/// 需登录的路由（统一鉴权后分发）
fn handle_authed(req: &HttpRequest, store: &mut Store) -> String {
    // 鉴权
    let token = match auth::token_from_cookie(&req.cookie) {
        Some(t) => t,
        None => return http::redirect("/login"),
    };
    let member_id = match auth::session_member_id(&token) {
        Some(id) => id,
        None => return http::redirect("/login"),
    };
    let member = match store.member_by_id(member_id) {
        Some(m) => m.clone(), // 克隆避免后续 &mut store 借用冲突
        None => return http::redirect("/login"),
    };

    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") => http_response_html(&page::home_html(&member)),
        // ---- PM 专属 ----
        ("GET", "/dashboard") => {
            pm_only(&member, || http_response_html(&page::dashboard_html(store)))
        }
        ("GET", "/projects") => pm_only(&member, || {
            http_response_html(&page::projects_html(store, None))
        }),
        ("POST", "/project") => pm_only(&member, || {
            create_project(req, store);
            http::redirect("/projects")
        }),
        ("GET", "/members") => pm_only(&member, || {
            http_response_html(&page::members_html(store, None))
        }),
        ("POST", "/member") => pm_only(&member, || {
            create_member(req, store);
            http::redirect("/members")
        }),
        // ---- 全员 ----
        ("GET", "/tasks") => http_response_html(&page::tasks_html(store, None)),
        ("POST", "/task") => {
            update_task(req, store);
            http::redirect("/tasks")
        }
        ("GET", "/timesheet") => http_response_html(&page::timesheet_form_html(store, member.id)),
        ("POST", "/timesheet") => {
            add_timesheet(req, store, member.id);
            http::redirect("/timesheet")
        }
        ("GET", "/my") => http_response_html(&page::my_page_html(store, member.id, &member.name)),
        _ => http::http_response(
            404,
            "text/html; charset=utf-8",
            &page::error_page("页面不存在"),
        ),
    }
}

/// PM 权限包装：非 PM 返回 403
fn pm_only<F: FnOnce() -> String>(member: &Member, f: F) -> String {
    if member.role.is_pm() {
        f()
    } else {
        http::http_response(
            403,
            "text/html; charset=utf-8",
            &page::error_page("权限不足：该功能仅项目经理可用"),
        )
    }
}

fn http_response_html(html: &str) -> String {
    http::http_response(200, "text/html; charset=utf-8", html)
}

// ============ 写操作（创建后统一 save） ============

fn create_project(req: &HttpRequest, store: &mut Store) {
    let project = Project {
        id: store.alloc_id(),
        name: req.param("name").unwrap_or_default(),
        desc: req.param("desc").unwrap_or_default(),
        budget: req
            .param("budget")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        start: req.param("start").unwrap_or_default(),
        deadline: req.param("deadline").unwrap_or_default(),
        status: consts::PRJ_ACTIVE.to_string(),
    };
    if !project.name.is_empty() {
        let _ = store.add_project(project);
        let _ = store.save();
    }
}

fn create_member(req: &HttpRequest, store: &mut Store) {
    let role = req
        .param("role")
        .and_then(|r| Role::from_str(&r))
        .unwrap_or(Role::Dev);
    let member = Member {
        id: store.alloc_id(),
        username: req.param("username").unwrap_or_default(),
        password_hash: auth::hash_password(&req.param("password").unwrap_or_default()),
        name: req.param("name").unwrap_or_default(),
        role,
        rate: req
            .param("rate")
            .and_then(|s| s.parse().ok())
            .unwrap_or(100.0),
    };
    if !member.username.is_empty() && !member.name.is_empty() {
        let _ = store.add_member(member);
        let _ = store.save();
    }
}

fn update_task(req: &HttpRequest, store: &mut Store) {
    // action=status&task_id=&status= 或 action=new
    let action = req.param("action").unwrap_or_default();
    if action == "status" {
        if let Some(task_id) = req.param("task_id").and_then(|s| s.parse().ok()) {
            if let Some(status) = req.param("status") {
                let _ = store.set_task_status(task_id, &status);
                let _ = store.save();
            }
        }
    } else if action == "new" {
        let task = Task {
            id: store.alloc_id(),
            project_id: req
                .param("project_id")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            title: req.param("title").unwrap_or_default(),
            assignee: req.param("assignee").and_then(|s| s.parse().ok()),
            priority: req
                .param("priority")
                .unwrap_or_else(|| consts::P_MEDIUM.to_string()),
            status: consts::T_TODO.to_string(),
            estimate_hours: req
                .param("estimate_hours")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            deadline: req.param("deadline").unwrap_or_default(),
        };
        if !task.title.is_empty() {
            let _ = store.add_task(task);
            let _ = store.save();
        }
    }
}

fn add_timesheet(req: &HttpRequest, store: &mut Store, member_id: u32) {
    let sheet = Timesheet {
        id: store.alloc_id(),
        task_id: req
            .param("task_id")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
        member_id,
        date: req.param("date").unwrap_or_else(Store::today_iso),
        hours: req
            .param("hours")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0),
        note: req.param("note").unwrap_or_default(),
    };
    if sheet.task_id > 0 && sheet.hours > 0.0 {
        let _ = store.add_timesheet(sheet);
        let _ = store.save();
    }
}
