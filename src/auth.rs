//! 认证与会话（成员D负责增强与测试；基线已可用）
//!
//! 设计说明：
//!   - 密码用 SHA-256 加固定盐哈希存储，明文不落盘；
//!     （教学演示方案；报告注明生产环境应使用 argon2/bcrypt）
//!   - 会话：登录成功生成 token（时间种子伪随机）存入内存表；
//!     通过 Cookie 下发浏览器，后续请求携带校验；
//!   - 服务重启后会话丢失（需重新登录），可接受。
//!
//! TODO(D)：补充登录失败尝试限制、会话过期时间、auth 单元测试、
//! 与 HTTP 登录流程集成测试。

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// 内存会话表：token -> member_id
fn sessions() -> &'static Mutex<HashMap<String, u32>> {
    static SESSIONS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

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
    sessions()
        .lock()
        .unwrap()
        .insert(token.clone(), member_id);
    token
}

/// 伪随机计数（时间种子的一部分）
fn rand_seed_counter() -> u64 {
    static COUNTER: OnceLock<std::sync::atomic::AtomicU64> = OnceLock::new();
    let c = COUNTER.get_or_init(|| std::sync::atomic::AtomicU64::new(0));
    c.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// 根据 token 查 member_id
pub fn session_member_id(token: &str) -> Option<u32> {
    sessions().lock().unwrap().get(token).copied()
}

/// 销毁会话（登出）
pub fn destroy_session(token: &str) {
    sessions().lock().unwrap().remove(token);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let h = hash_password("pm123");
        assert_ne!(h, "pm123"); // 不是明文
        assert!(verify_password("pm123", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn session_create_and_query() {
        let token = create_session(1);
        assert_eq!(session_member_id(&token), Some(1));
        destroy_session(&token);
        assert_eq!(session_member_id(&token), None);
    }

    #[test]
    fn cookie_token_parse() {
        assert_eq!(token_from_cookie("token=abc; foo=1"), Some("abc".to_string()));
        assert_eq!(token_from_cookie("foo=1; token=xyz"), Some("xyz".to_string()));
        assert_eq!(token_from_cookie("foo=1"), None);
    }
}
