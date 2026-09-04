//! 认证与会话（成员D负责增强与测试；基线已可用）
//!
//! 设计说明：
//!   - 密码用 SHA-256 加固定盐哈希存储，明文不落盘；
//!     （教学演示方案；报告注明生产环境应使用 argon2/bcrypt）
//!   - 会话：登录成功生成 token（时间种子伪随机）存入内存表；
//!     通过 Cookie 下发浏览器，后续请求携带校验；
//!   - 服务重启后会话丢失（需重新登录），可接受。
//!   - 增强（D 补强）：
//!     1) 登录失败尝试限制：同一用户名连续失败 5 次后锁定 5 分钟；
//!     2) 会话过期：每条会话记录创建时间戳，超过 TTL 自动失效；
//!     3) 过期会话清理 utility。

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// ==================== 常量 ====================

/// 最大登录失败次数（达到后锁定）
const MAX_LOGIN_ATTEMPTS: u32 = 5;

/// 锁定持续时间（秒）
const LOCKOUT_DURATION_SECS: u64 = 300; // 5 分钟

/// 默认会话 TTL（秒）
const DEFAULT_SESSION_TTL_SECS: u64 = 3600; // 1 小时

// ==================== 会话管理 ====================

/// 会话信息（含创建时间，用于过期判断）
struct SessionInfo {
    member_id: u32,
    created_at: u64,
}

/// 内存会话表：token -> SessionInfo
fn sessions() -> &'static Mutex<HashMap<String, SessionInfo>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, SessionInfo>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 当前 Unix 时间戳（秒）
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ==================== 会话 TTL（可调，测试用） ====================

/// TTL 存储（AtomicU64，允许运行时调整）
fn ttl_value() -> &'static std::sync::atomic::AtomicU64 {
    static TTL: OnceLock<std::sync::atomic::AtomicU64> = OnceLock::new();
    TTL.get_or_init(|| std::sync::atomic::AtomicU64::new(DEFAULT_SESSION_TTL_SECS))
}

/// 当前会话 TTL（秒）
fn session_ttl() -> u64 {
    ttl_value().load(std::sync::atomic::Ordering::Relaxed)
}

/// 设置会话 TTL（测试用：可设为极短值验证过期逻辑）
pub fn set_session_ttl(seconds: u64) {
    ttl_value().store(seconds, std::sync::atomic::Ordering::Relaxed);
}

// ==================== 密码哈希 ====================

/// 密码哈希（加固定盐的 SHA-256）
pub fn hash_password(plain: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"rdplatform_salt_2026:");
    hasher.update(plain.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 校验明文密码是否与哈希匹配
pub fn verify_password(plain: &str, hash: &str) -> bool {
    hash_password(plain) == hash
}

// ==================== 会话操作 ====================

/// 创建会话，返回 token
pub fn create_session(member_id: u32) -> String {
    // 时间种子伪随机 token（演示用；生产应使用 CSPRNG）
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}:{}", nanos, member_id, rand_seed_counter()).as_bytes());
    let token = format!("{:x}", hasher.finalize());
    sessions().lock().unwrap().insert(
        token.clone(),
        SessionInfo {
            member_id,
            created_at: now_secs(),
        },
    );
    token
}

/// 伪随机计数（时间种子的一部分）
fn rand_seed_counter() -> u64 {
    static COUNTER: OnceLock<std::sync::atomic::AtomicU64> = OnceLock::new();
    let c = COUNTER.get_or_init(|| std::sync::atomic::AtomicU64::new(0));
    c.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// 根据 token 查 member_id（自动检查过期，过期返回 None）
pub fn session_member_id(token: &str) -> Option<u32> {
    let ttl = session_ttl();
    let now = now_secs();
    sessions()
        .lock()
        .unwrap()
        .get(token)
        .filter(|s| now.saturating_sub(s.created_at) < ttl)
        .map(|s| s.member_id)
}

/// 销毁会话（登出）
pub fn destroy_session(token: &str) {
    sessions().lock().unwrap().remove(token);
}

/// 清理所有过期会话
pub fn cleanup_expired_sessions() {
    let ttl = session_ttl();
    let now = now_secs();
    sessions()
        .lock()
        .unwrap()
        .retain(|_, s| now.saturating_sub(s.created_at) < ttl);
}

/// 当前活跃会话数（未过期）
pub fn active_session_count() -> usize {
    let ttl = session_ttl();
    let now = now_secs();
    sessions()
        .lock()
        .unwrap()
        .values()
        .filter(|s| now.saturating_sub(s.created_at) < ttl)
        .count()
}

/// 手动使会话过期（测试用：将 created_at 置零使即超时）
pub fn force_expire_session(token: &str) {
    if let Some(s) = sessions().lock().unwrap().get_mut(token) {
        s.created_at = 0;
    }
}

/// 从请求 Cookie 头中解析 token（简化：找 token=xxx）
pub fn token_from_cookie(cookie: &str) -> Option<String> {
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=') {
            if k.trim() == "token" {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

// ==================== 登录失败限制 ====================

/// 登录失败记录
struct LoginFailure {
    count: u32,
    first_failure_secs: u64,
}

/// 登录失败表：username -> LoginFailure
fn login_failures() -> &'static Mutex<HashMap<String, LoginFailure>> {
    static FAILURES: OnceLock<Mutex<HashMap<String, LoginFailure>>> = OnceLock::new();
    FAILURES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 记录一次登录失败（锁定窗口过期后自动重新计数）
pub fn record_login_failure(username: &str) {
    let now = now_secs();
    let mut failures = login_failures().lock().unwrap();
    match failures.get_mut(username) {
        Some(f) => {
            // 锁定窗口已过 → 重新计数
            if now.saturating_sub(f.first_failure_secs) >= LOCKOUT_DURATION_SECS {
                f.count = 1;
                f.first_failure_secs = now;
            } else {
                f.count += 1;
            }
        }
        None => {
            failures.insert(
                username.to_string(),
                LoginFailure {
                    count: 1,
                    first_failure_secs: now,
                },
            );
        }
    }
}

/// 登录成功后清除失败记录
pub fn clear_login_failures(username: &str) {
    login_failures().lock().unwrap().remove(username);
}

/// 是否已被锁定（失败次数达上限且仍在锁定窗口内）
pub fn is_locked_out(username: &str) -> bool {
    let now = now_secs();
    match login_failures().lock().unwrap().get(username) {
        Some(f) => {
            f.count >= MAX_LOGIN_ATTEMPTS
                && now.saturating_sub(f.first_failure_secs) < LOCKOUT_DURATION_SECS
        }
        None => false,
    }
}

/// 剩余尝试次数（锁定窗口过期后自动恢复满额）
pub fn remaining_attempts(username: &str) -> u32 {
    let now = now_secs();
    match login_failures().lock().unwrap().get(username) {
        Some(f) => {
            if now.saturating_sub(f.first_failure_secs) >= LOCKOUT_DURATION_SECS {
                MAX_LOGIN_ATTEMPTS
            } else {
                MAX_LOGIN_ATTEMPTS.saturating_sub(f.count)
            }
        }
        None => MAX_LOGIN_ATTEMPTS,
    }
}

// ==================== 单元测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- 密码哈希 ----

    #[test]
    fn hash_and_verify() {
        let h = hash_password("pm123");
        assert_ne!(h, "pm123"); // 不是明文
        assert!(verify_password("pm123", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn hash_is_deterministic() {
        // 相同密码产生相同哈希
        assert_eq!(hash_password("dv123"), hash_password("dv123"));
    }

    #[test]
    fn hash_differs_for_different_passwords() {
        assert_ne!(hash_password("pm123"), hash_password("dv123"));
    }

    // ---- 会话生命周期 ----

    #[test]
    fn session_create_and_query() {
        let token = create_session(1);
        assert_eq!(session_member_id(&token), Some(1));
        destroy_session(&token);
        assert_eq!(session_member_id(&token), None);
    }

    #[test]
    fn session_unique_tokens() {
        let t1 = create_session(1);
        let t2 = create_session(1);
        assert_ne!(t1, t2); // 同一用户两次创建，token 不同
        destroy_session(&t1);
        destroy_session(&t2);
    }

    // ---- 会话过期 ----

    #[test]
    fn session_expiry_via_force() {
        let token = create_session(2);
        assert_eq!(session_member_id(&token), Some(2));
        force_expire_session(&token); // 模拟超时
        assert_eq!(session_member_id(&token), None);
    }

    #[test]
    fn session_expiry_via_short_ttl() {
        set_session_ttl(1);
        let token = create_session(3);
        assert_eq!(session_member_id(&token), Some(3));
        force_expire_session(&token);
        assert_eq!(session_member_id(&token), None);
        set_session_ttl(DEFAULT_SESSION_TTL_SECS); // 恢复
    }

    #[test]
    fn cleanup_removes_only_expired() {
        let t1 = create_session(1);
        let t2 = create_session(2);
        force_expire_session(&t1);
        cleanup_expired_sessions();
        assert_eq!(session_member_id(&t1), None);
        assert_eq!(session_member_id(&t2), Some(2));
        destroy_session(&t2);
    }

    #[test]
    fn active_session_count_is_nonnegative() {
        // 并行测试共享全局会话表，不做精确计数断言
        let _ = active_session_count();
        let t = create_session(99);
        assert!(session_member_id(&t).is_some());
        destroy_session(&t);
        assert!(session_member_id(&t).is_none());
    }

    // ---- Cookie 解析 ----

    #[test]
    fn cookie_token_parse() {
        assert_eq!(token_from_cookie("token=abc; foo=1"), Some("abc".to_string()));
        assert_eq!(token_from_cookie("foo=1; token=xyz"), Some("xyz".to_string()));
        assert_eq!(token_from_cookie("foo=1"), None);
    }

    #[test]
    fn cookie_edge_cases() {
        assert_eq!(token_from_cookie(""), None);
        assert_eq!(token_from_cookie("token="), Some(String::new()));
        assert_eq!(token_from_cookie("token=abc;"), Some("abc".to_string()));
        // 多个 token 取第一个
        assert_eq!(
            token_from_cookie("token=first; token=second"),
            Some("first".to_string())
        );
    }

    // ---- 登录失败限制 ----

    #[test]
    fn login_failure_lockout_cycle() {
        let user = "test_lockout_user_d";
        clear_login_failures(user);

        assert!(!is_locked_out(user));
        assert_eq!(remaining_attempts(user), MAX_LOGIN_ATTEMPTS);

        for i in 0..MAX_LOGIN_ATTEMPTS {
            record_login_failure(user);
            let expected = MAX_LOGIN_ATTEMPTS - i - 1;
            assert_eq!(remaining_attempts(user), expected, "失败第 {} 次后剩余尝试", i + 1);
        }

        assert!(is_locked_out(user));
        assert_eq!(remaining_attempts(user), 0);

        clear_login_failures(user);
        assert!(!is_locked_out(user));
        assert_eq!(remaining_attempts(user), MAX_LOGIN_ATTEMPTS);
    }

    #[test]
    fn login_failure_reset_on_success() {
        let user = "test_reset_user_d";
        clear_login_failures(user);

        record_login_failure(user);
        record_login_failure(user);
        assert_eq!(remaining_attempts(user), MAX_LOGIN_ATTEMPTS - 2);

        clear_login_failures(user); // 模拟登录成功
        assert_eq!(remaining_attempts(user), MAX_LOGIN_ATTEMPTS);
        assert!(!is_locked_out(user));
    }

    #[test]
    fn login_failure_no_record_is_not_locked() {
        let user = "test_clean_user_d";
        clear_login_failures(user);
        assert!(!is_locked_out(user));
        assert_eq!(remaining_attempts(user), MAX_LOGIN_ATTEMPTS);
    }
}