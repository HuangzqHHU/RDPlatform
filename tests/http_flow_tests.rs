//! HTTP 登录/权限集成测试（成员D负责）
//!
//! 测试范围：
//!   - 登录成功流程：POST /login → 302 + Set-Cookie
//!   - 登录失败流程：POST /login 错误密码 → 200 + 错误提示
//!   - 未登录访问受保护页面 → 302 重定向到 /login
//!   - 权限分级：dev 访问 /dashboard → 403；pm 访问 → 200
//!   - 全员路由：dev 访问 /tasks → 200
//!   - 登出流程：GET /logout → 302 重定向到 /login
//!   - Cookie 会话：登录后携带 Cookie 访问首页 → 200

use rdplatform::api;
use rdplatform::auth;
use rdplatform::http::HttpRequest;
use rdplatform::store::Store;

// ==================== 辅助函数 ====================

/// 构造 POST /login 请求
fn login_request(username: &str, password: &str) -> HttpRequest {
    HttpRequest {
        method: "POST".into(),
        path: "/login".into(),
        query: vec![],
        body: vec![
            ("username".into(), username.into()),
            ("password".into(), password.into()),
        ],
        cookie: String::new(),
    }
}

/// 构造 GET 请求（可带 Cookie）
fn get_request(path: &str, cookie: &str) -> HttpRequest {
    HttpRequest {
        method: "GET".into(),
        path: path.into(),
        query: vec![],
        body: vec![],
        cookie: cookie.into(),
    }
}

/// 从 HTTP 响应中提取 Set-Cookie 里的 token 值
fn extract_token(resp: &str) -> Option<String> {
    for line in resp.split("\r\n") {
        if line.to_lowercase().starts_with("set-cookie:") {
            let value = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            for part in value.split(';') {
                let part = part.trim();
                if let Some((k, v)) = part.split_once('=') {
                    if k.trim() == "token" {
                        return Some(v.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

/// 带有种子数据的测试 Store
fn test_store() -> Store {
    let mut s = Store::new();
    s.path = "data/test_http.json".into();
    s.seed();
    s
}

// ==================== 登录成功 ====================

#[test]
fn login_success_returns_redirect_with_cookie() {
    let mut store = test_store();
    let req = login_request("pm1", "pm123");
    let resp = api::dispatch(&req, &mut store);

    // 302 重定向
    assert!(resp.starts_with("HTTP/1.1 302"), "应返回 302 重定向");
    assert!(resp.contains("Location: /"), "应重定向到首页");

    // Set-Cookie 包含 token
    let token = extract_token(&resp);
    assert!(token.is_some(), "响应应包含 Set-Cookie token");
    assert!(!token.unwrap().is_empty(), "token 不能为空");
}

#[test]
fn login_success_dev_account() {
    let mut store = test_store();
    let req = login_request("dev1", "dv123");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.starts_with("HTTP/1.1 302"));
    assert!(extract_token(&resp).is_some());
}

#[test]
fn login_success_qa_account() {
    let mut store = test_store();
    let req = login_request("qa1", "qa123");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.starts_with("HTTP/1.1 302"));
    assert!(extract_token(&resp).is_some());
}

// ==================== 登录失败 ====================

#[test]
fn login_wrong_password_returns_error_page() {
    let mut store = test_store();
    let req = login_request("pm1", "wrong_password");
    let resp = api::dispatch(&req, &mut store);

    // 200 + 错误提示（非重定向）
    assert!(resp.starts_with("HTTP/1.1 200"), "应返回 200");
    assert!(!resp.contains("Set-Cookie:"), "失败不应下发 Cookie");
    assert!(resp.contains("错误"), "应包含错误提示");
}

#[test]
fn login_unknown_user_returns_error() {
    let mut store = test_store();
    let req = login_request("nobody", "whatever");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.starts_with("HTTP/1.1 200"));
    assert!(!resp.contains("Set-Cookie:"));
    assert!(resp.contains("错误"));
}

#[test]
fn login_empty_credentials_returns_error() {
    let mut store = test_store();
    let req = login_request("", "");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.starts_with("HTTP/1.1 200"));
    assert!(!resp.contains("Set-Cookie:"));
}

// ==================== 未登录访问受保护页面 ====================

#[test]
fn access_without_login_redirects_to_login() {
    let mut store = test_store();
    let req = get_request("/", "");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.starts_with("HTTP/1.1 302"), "应重定向");
    assert!(resp.contains("Location: /login"), "应重定向到登录页");
}

#[test]
fn access_dashboard_without_login_redirects() {
    let mut store = test_store();
    let req = get_request("/dashboard", "");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.contains("Location: /login"));
}

#[test]
fn access_tasks_without_login_redirects() {
    let mut store = test_store();
    let req = get_request("/tasks", "");
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.contains("Location: /login"));
}

// ==================== 权限分级测试 ====================

#[test]
fn dev_access_dashboard_gets_403() {
    let mut store = test_store();

    // dev1 登录
    let login_req = login_request("dev1", "dv123");
    let login_resp = api::dispatch(&login_req, &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    // dev1 访问 /dashboard → 403
    let req = get_request("/dashboard", &cookie);
    let resp = api::dispatch(&req, &mut store);

    assert!(resp.starts_with("HTTP/1.1 403"), "dev 访问仪表盘应返回 403");
    assert!(resp.contains("权限不足"), "应提示权限不足");

    // 清理会话
    auth::destroy_session(&token);
}

#[test]
fn qa_access_dashboard_gets_403() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("qa1", "qa123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/dashboard", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 403"));

    auth::destroy_session(&token);
}

#[test]
fn pm_access_dashboard_gets_200() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("pm1", "pm123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/dashboard", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"), "PM 访问仪表盘应返回 200");

    auth::destroy_session(&token);
}

#[test]
fn dev_access_projects_gets_403() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("dev1", "dv123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/projects", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 403"));

    auth::destroy_session(&token);
}

#[test]
fn pm_access_projects_gets_200() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("pm1", "pm123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/projects", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"));

    auth::destroy_session(&token);
}

// ==================== 全员路由 ====================

#[test]
fn dev_access_tasks_gets_200() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("dev1", "dv123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/tasks", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"), "全员路由 /tasks 应返回 200");

    auth::destroy_session(&token);
}

#[test]
fn dev_access_my_page_gets_200() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("dev1", "dv123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/my", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"));

    auth::destroy_session(&token);
}

#[test]
fn qa_access_timesheet_gets_200() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("qa1", "qa123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/timesheet", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"));

    auth::destroy_session(&token);
}

// ==================== 首页访问（登录后） ====================

#[test]
fn logged_in_access_home_gets_200() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("pm1", "pm123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"));

    auth::destroy_session(&token);
}

// ==================== 登出流程 ====================

#[test]
fn logout_redirects_to_login() {
    let mut store = test_store();

    // 先登录
    let login_resp = api::dispatch(&login_request("pm1", "pm123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    // 登出
    let resp = api::dispatch(&get_request("/logout", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 302"), "登出应重定向");
    assert!(resp.contains("Location: /login"), "应重定向到登录页");

    // 登出后会话失效，再访问首页应重定向到登录
    let resp2 = api::dispatch(&get_request("/", &cookie), &mut store);
    assert!(resp2.contains("Location: /login"), "登出后访问应重定向到登录");
}

// ==================== 无效路由 ====================

#[test]
fn unknown_path_returns_404() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("pm1", "pm123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    let resp = api::dispatch(&get_request("/nonexistent", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 404"), "未知路由应返回 404");

    auth::destroy_session(&token);
}

// ==================== 会话过期后访问 ====================

#[test]
fn expired_session_redirects_to_login() {
    let mut store = test_store();

    let login_resp = api::dispatch(&login_request("pm1", "pm123"), &mut store);
    let token = extract_token(&login_resp).unwrap();
    let cookie = format!("token={}", token);

    // 先验证可正常访问
    let resp = api::dispatch(&get_request("/", &cookie), &mut store);
    assert!(resp.starts_with("HTTP/1.1 200"));

    // 使会话过期
    auth::force_expire_session(&token);

    // 过期后访问应重定向到登录
    let resp = api::dispatch(&get_request("/", &cookie), &mut store);
    assert!(resp.contains("Location: /login"), "过期会话应重定向到登录");

    auth::destroy_session(&token);
}
