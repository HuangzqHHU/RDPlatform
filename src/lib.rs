//! rdplatform —— 研发任务与工时管理平台（企业研发团队内部管理工具）
//!
//! 模块划分（分工见 RDP-PLAN.md）：
//!   model      数据模型（B）
//!   store      JSON 持久化 + CRUD + 统计（B）
//!   http       HTTP 协议层（A，已完整实现）
//!   auth       账号密码登录与会话权限（D）
//!   api        路由分发与业务 handler（A）
//!   page       HTML 页面生成（C）

pub mod api;
pub mod auth;
pub mod http;
pub mod model;
pub mod page;
pub mod store;

/// 当前版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
