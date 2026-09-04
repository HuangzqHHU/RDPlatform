//! auth 模块集成测试（成员D负责）
//!
//! 测试范围：
//!   - 密码哈希与校验（与 store seed 数据交叉验证）
//!   - 会话生命周期：创建 → 查询 → 销毁
//!   - 会话过期：force_expire + 短 TTL
//!   - 登录失败限制：失败计数 → 锁定 → 恢复
//!   - Cookie 解析（HTTP 层交互）

use rdplatform::auth;
use rdplatform::store::Store;

// ==================== 密码哈希（与 seed 数据交叉验证） ====================

#[test]
fn seeded_password_hashes_match() {
    // store.seed() 写入的密码哈希必须能被 auth::verify_password 校验通过
    let mut s = Store::new();
    s.seed();

    let pm = s.find_member_by_username("pm1").unwrap();
    assert!(auth::verify_password("pm123", &pm.password_hash));
    assert!(!auth::verify_password("wrong", &pm.password_hash));

    let dev = s.find_member_by_username("dev1").unwrap();
    assert!(auth::verify_password("dv123", &dev.password_hash));

    let qa = s.find_member_by_username("qa1").unwrap();
    assert!(auth::verify_password("qa123", &qa.password_hash));
}

#[test]
fn hash_password_never_returns_plain() {
    for pw in &["pm123", "dv123", "qa123", "", "a"] {
        let h = auth::hash_password(pw);
        assert_ne!(h, *pw, "哈希值不能等于明文");
        assert!(h.len() > 20, "哈希值长度应 > 20");
    }
}

// ==================== 会话生命周期 ====================

#[test]
fn session_full_lifecycle() {
    let token = auth::create_session(1);
    assert!(auth::session_member_id(&token).is_some());

    // 销毁后不可查询
    auth::destroy_session(&token);
    assert!(auth::session_member_id(&token).is_none());
}

#[test]
fn multiple_sessions_same_member() {
    let t1 = auth::create_session(5);
    let t2 = auth::create_session(5);
    assert_ne!(t1, t2);
    assert_eq!(auth::session_member_id(&t1), Some(5));
    assert_eq!(auth::session_member_id(&t2), Some(5));

    // 销毁一个不影响另一个
    auth::destroy_session(&t1);
    assert!(auth::session_member_id(&t1).is_none());
    assert_eq!(auth::session_member_id(&t2), Some(5));
    auth::destroy_session(&t2);
}

#[test]
fn unknown_token_returns_none() {
    assert!(auth::session_member_id("nonexistent_token_xyz").is_none());
}

// ==================== 会话过期 ====================

#[test]
fn expired_session_is_invalid() {
    let token = auth::create_session(1);
    assert!(auth::session_member_id(&token).is_some());

    auth::force_expire_session(&token);
    assert!(auth::session_member_id(&token).is_none());
}

#[test]
fn cleanup_does_not_affect_active_sessions() {
    let t1 = auth::create_session(1);
    let t2 = auth::create_session(2);
    auth::force_expire_session(&t1);

    auth::cleanup_expired_sessions();
    assert!(auth::session_member_id(&t1).is_none());
    assert!(auth::session_member_id(&t2).is_some());

    auth::destroy_session(&t2);
}

// ==================== Cookie 解析 ====================

#[test]
fn cookie_parse_various_formats() {
    // 标准 cookie
    assert_eq!(
        auth::token_from_cookie("token=abc123; other=xyz"),
        Some("abc123".to_string())
    );
    // token 在末尾
    assert_eq!(
        auth::token_from_cookie("lang=zh; token=xyz789"),
        Some("xyz789".to_string())
    );
    // 只有 token
    assert_eq!(
        auth::token_from_cookie("token=solo"),
        Some("solo".to_string())
    );
}

#[test]
fn cookie_parse_missing_token() {
    assert!(auth::token_from_cookie("").is_none());
    assert!(auth::token_from_cookie("lang=zh; theme=dark").is_none());
    assert!(auth::token_from_cookie("notoken=abc").is_none());
}

// ==================== 登录失败限制 ====================

#[test]
fn login_failure_count_and_lockout() {
    let user = "integ_lockout_user";
    auth::clear_login_failures(user);

    assert!(!auth::is_locked_out(user));
    assert_eq!(auth::remaining_attempts(user), 5);

    // 失败 4 次：未锁定
    for _ in 0..4 {
        auth::record_login_failure(user);
    }
    assert!(!auth::is_locked_out(user));
    assert_eq!(auth::remaining_attempts(user), 1);

    // 第 5 次失败：锁定
    auth::record_login_failure(user);
    assert!(auth::is_locked_out(user));
    assert_eq!(auth::remaining_attempts(user), 0);

    auth::clear_login_failures(user);
}

#[test]
fn login_success_clears_failures() {
    let user = "integ_success_user";
    auth::clear_login_failures(user);

    auth::record_login_failure(user);
    auth::record_login_failure(user);
    assert_eq!(auth::remaining_attempts(user), 3);

    // 模拟登录成功
    auth::clear_login_failures(user);
    assert!(!auth::is_locked_out(user));
    assert_eq!(auth::remaining_attempts(user), 5);
}

#[test]
fn unknown_user_has_full_attempts() {
    let user = "integ_unknown_user";
    auth::clear_login_failures(user);
    assert_eq!(auth::remaining_attempts(user), 5);
    assert!(!auth::is_locked_out(user));
}
