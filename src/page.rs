//! HTML 页面生成（成员C负责）
//!
//! 契约（勿改签名）：所有页面函数返回**页面主体 HTML 字符串**（不含 HTTP 头，
//! 由 api.rs 调用 http::http_response 包装）。页面内部数据一律先 http::html_escape。
//!
//! 约定：
//!   - 登录页/错误提示页用 http::html_escape 处理用户输入；
//!   - layout() 是导航壳（顶部身份条 + 按角色显示的导航菜单），各页面复用；
//!   - 页面数据来源：store 的方法（B 实现）——进度/成本/负载/逾期等统计
//!     已在 store.rs 定义，直接调用即可；
//!   - 表单提交路径见 RDP-PLAN.md 路由表。
//!
//! TODO(C)：逐个实现以下页面（可先实现 layout 与登录页让登录流程先通）。

use crate::http::html_escape;
use crate::store::Store;

/// 登录页（error: 登录失败提示）
pub fn login_page(error: Option<&str>) -> String {
    let err_html = match error {
        Some(e) => format!("<p style='color:red'>{}</p>", html_escape(e)),
        None => String::new(),
    };
    format!(
        r#"<!DOCTYPE html><html lang="zh"><head><meta charset="utf-8">
<title>RDPlatform 登录</title></head><body style="font-family:微软雅黑;text-align:center;margin-top:80px;">
<h1>RDPlatform 研发任务与工时管理平台</h1>
{}
<form method="post" action="/login" style="display:inline-block;text-align:left;">
  用户名: <input name="username"><br><br>
  密　码: <input type="password" name="password"><br><br>
  <button type="submit">登 录</button>
</form>
<p style="color:#888;margin-top:30px;">演示账号：pm1/pm123（项目经理）　dev1/dv123（开发）　qa1/qa123（测试）</p>
</body></html>"#,
        err_html
    )
}

/// 导航壳（顶部身份条 + 按角色显示菜单；content 为页面主体）
pub fn layout(title: &str, member_name: &str, role: &str, content: &str) -> String {
    // TODO(C)：完善菜单与样式（/projects /tasks /timesheet /my /dashboard /members）
    let nav = format!(
        "RDPlatform | 当前用户：{}（{}） <a href='/'>首页</a> | <a href='/logout'>登出</a>",
        html_escape(member_name),
        html_escape(role)
    );
    format!(
        r#"<!DOCTYPE html><html lang="zh"><head><meta charset="utf-8"><title>{}</title></head>
<body style="font-family:微软雅黑;">
<div style="background:#eee;padding:8px;">{}</div>
<h2>{}</h2>
{}</body></html>"#,
        html_escape(title),
        nav,
        html_escape(title),
        content
    )
}

/// 管理仪表盘（PM）：进度/成本对比预算/估算偏差/成员负载/逾期任务
pub fn dashboard_html(store: &Store) -> String {
    // TODO(C)：调用 store 的 5 项统计（store::project_progress / project_cost /
    //          estimate_deviation / member_load / overdue_tasks），渲染表格。
    let _ = store;
    layout("管理仪表盘", "PM", "项目经理", "<p>仪表盘待实现</p>")
}

/// 项目列表页 + 新建项目表单
pub fn projects_html(store: &Store, flash: Option<&str>) -> String {
    let _ = (store, flash);
    layout("项目管理", "PM", "项目经理", "<p>项目列表待实现</p>")
}

/// 任务列表页（含筛选/新建任务表单/状态流转按钮）
pub fn tasks_html(store: &Store, flash: Option<&str>) -> String {
    let _ = (store, flash);
    layout("任务管理", "PM", "项目经理", "<p>任务列表待实现</p>")
}

/// 工时登记页（选任务 + 小时数 + 备注；累计超预估预警）
pub fn timesheet_form_html(store: &Store, member_id: u32) -> String {
    let _ = (store, member_id);
    layout("工时登记", "成员", "开发", "<p>工时登记待实现</p>")
}

/// 我的视图（我的任务 + 我的工时）
pub fn my_page_html(store: &Store, member_id: u32, member_name: &str) -> String {
    let _ = (store, member_id, member_name);
    layout("我的工作台", "成员", "成员", "<p>我的视图待实现</p>")
}

/// 成员列表（PM：查看/新增成员、设定费率）
pub fn members_html(store: &Store, flash: Option<&str>) -> String {
    let _ = (store, flash);
    layout("成员管理", "PM", "项目经理", "<p>成员管理待实现</p>")
}

/// 简单错误页
pub fn error_page(message: &str) -> String {
    format!("<h1>出错了</h1><p>{}</p><p><a href='/'>返回首页</a></p>", html_escape(message))
}

/// 首页：按角色的快捷入口
pub fn home_html(member: &crate::model::Member) -> String {
    // TODO(C)：完善首页（我的任务入口 / 快捷链接）
    let _ = member;
    layout("首页", "用户", "角色", "<p>首页待实现（我的任务/快捷入口）</p>")
}
