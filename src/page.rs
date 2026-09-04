//! RDPlatform HTML 页面生成（成员 C）

use crate::http::html_escape;
use crate::model::{consts, Member};
use crate::store::Store;

/// 显示操作提示
fn flash_html(flash: Option<&str>) -> String {
    match flash {
        Some(message) => {
            format!(
                "<p class='warning'>{}</p>",
                html_escape(message)
            )
        }
        None => String::new(),
    }
}

/// 所有登录后页面共用的页面布局
pub fn layout(
    title: &str,
    member_name: &str,
    role: &str,
    content: &str,
) -> String {
    let mut nav = String::new();

    nav.push_str("<a href='/'>首页</a> | ");
    nav.push_str("<a href='/tasks'>任务管理</a> | ");
    nav.push_str("<a href='/timesheet'>工时登记</a> | ");
    nav.push_str("<a href='/my'>我的工作台</a> | ");

    if role == "项目经理" || role.eq_ignore_ascii_case("pm") {
        nav.push_str("<a href='/dashboard'>管理仪表盘</a> | ");
        nav.push_str("<a href='/projects'>项目管理</a> | ");
        nav.push_str("<a href='/members'>成员管理</a> | ");
    }

    nav.push_str("<a href='/logout'>退出登录</a>");

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{}</title>

<style>
body {{
    margin: 0;
    background: #f4f6f8;
    color: #333;
    font-family: "Microsoft YaHei", sans-serif;
}}

header {{
    padding: 20px 30px;
    background: #1f4e79;
    color: white;
}}

header h1 {{
    margin: 0 0 8px 0;
}}

nav {{
    padding: 14px 30px;
    background: white;
    border-bottom: 1px solid #ddd;
}}

nav a {{
    margin-right: 10px;
    color: #1f4e79;
    text-decoration: none;
}}

main {{
    padding: 25px 30px;
}}

.card {{
    margin-bottom: 20px;
    padding: 18px;
    background: white;
    border-radius: 8px;
    box-shadow: 0 1px 4px #ddd;
}}

table {{
    width: 100%;
    border-collapse: collapse;
    background: white;
    margin: 12px 0 20px 0;
}}

th, td {{
    padding: 9px;
    border: 1px solid #ddd;
    text-align: left;
}}

th {{
    background: #eaf2f8;
}}

input, select, textarea {{
    margin: 4px;
    padding: 7px;
}}

button {{
    padding: 7px 16px;
    cursor: pointer;
}}

.warning {{
    color: #b45309;
    font-weight: bold;
}}

.danger {{
    color: #dc2626;
    font-weight: bold;
}}

.ok {{
    color: #15803d;
    font-weight: bold;
}}

.muted {{
    color: #777;
}}
</style>
</head>

<body>
<header>
    <h1>研发任务与工时管理平台</h1>
    <div>当前用户：{}　角色：{}</div>
</header>

<nav>{}</nav>

<main>
    <h2>{}</h2>
    {}
</main>
</body>
</html>"#,
        html_escape(title),
        html_escape(member_name),
        html_escape(role),
        nav,
        html_escape(title),
        content,
    )
}

/// 登录页面
pub fn login_page(error: Option<&str>) -> String {
    let error_html = match error {
        Some(message) => {
            format!(
                "<p class='danger'>{}</p>",
                html_escape(message)
            )
        }
        None => String::new(),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<title>RDPlatform 登录</title>
</head>

<body style="font-family:Microsoft YaHei;text-align:center;background:#f4f6f8;padding-top:80px;">

<div style="display:inline-block;background:white;padding:35px 50px;border-radius:10px;box-shadow:0 1px 5px #ddd;">
    <h1>研发任务与工时管理平台</h1>

    {}

    <form method="post" action="/login">
        <p>
            用户名：
            <input name="username" required>
        </p>

        <p>
            密　码：
            <input type="password" name="password" required>
        </p>

        <button type="submit">登 录</button>
    </form>

    <p style="color:#777;">
        演示账号：pm1/pm123　dev1/dv123　qa1/qa123
    </p>
</div>

</body>
</html>"#,
        error_html
    )
}

/// 首页
pub fn home_html(member: &Member) -> String {
    let role = member.role.as_str();
    let mut content = String::new();

    content.push_str("<div class='card'>");
    content.push_str("<h3>欢迎回来</h3>");
    content.push_str("<p>请选择需要使用的功能。</p>");

    if member.role.is_pm() {
        content.push_str("<p><a href='/dashboard'>进入管理仪表盘</a></p>");
        content.push_str("<p><a href='/projects'>项目管理</a></p>");
        content.push_str("<p><a href='/members'>成员管理</a></p>");
    }

    content.push_str("<p><a href='/tasks'>查看任务</a></p>");
    content.push_str("<p><a href='/my'>查看我的工作台</a></p>");

    if !member.role.is_pm() {
        content.push_str("<p><a href='/timesheet'>登记工时</a></p>");
    }

    content.push_str("</div>");

    layout("首页", &member.name, role, &content)
}

/// PM 管理仪表盘
pub fn dashboard_html(store: &Store) -> String {
    let mut content = String::new();

    content.push_str("<div class='card'>");
    content.push_str("<h3>项目统计</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>项目</th>
            <th>进度</th>
            <th>实际成本 / 预算</th>
            <th>估算偏差</th>
        </tr>",
    );

    for project in &store.projects {
        let (finished, total) =
            store.project_progress(project.id);

        let progress = if total == 0 {
            0.0
        } else {
            finished as f64 / total as f64 * 100.0
        };

        let cost = store.project_cost(project.id);

        let budget_ratio = if project.budget <= 0.0 {
            0.0
        } else {
            cost / project.budget
        };

        let cost_class = if budget_ratio >= 1.0 {
            "danger"
        } else if budget_ratio >= 0.8 {
            "warning"
        } else {
            "ok"
        };

        let deviation = store.estimate_deviation(project.id);

        let deviation_class = if deviation > 0.0 {
            "warning"
        } else {
            "ok"
        };

        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{} / {}（{:.1}%）</td>
                <td class='{}'>{:.2} / {:.2}</td>
                <td class='{}'>{:+.1} 小时</td>
            </tr>",
            html_escape(&project.name),
            finished,
            total,
            progress,
            cost_class,
            cost,
            project.budget,
            deviation_class,
            deviation,
        ));
    }

    if store.projects.is_empty() {
        content.push_str(
            "<tr>
                <td colspan='4' class='muted'>暂无项目</td>
            </tr>",
        );
    }

    content.push_str("</table>");
    content.push_str("</div>");

    content.push_str("<div class='card'>");
    content.push_str("<h3>成员负载</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>成员 ID</th>
            <th>姓名</th>
            <th>累计工时</th>
        </tr>",
    );

    for (member_id, name, hours) in store.member_load() {
        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{:.1} 小时</td>
            </tr>",
            member_id,
            html_escape(&name),
            hours,
        ));
    }

    content.push_str("</table>");
    content.push_str("</div>");

    content.push_str("<div class='card'>");
    content.push_str("<h3>逾期未完成任务</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>任务</th>
            <th>截止日期</th>
            <th>状态</th>
        </tr>",
    );

    let today = Store::today_iso();
    let overdue_tasks = store.overdue_tasks(&today);

    for task in overdue_tasks {
        content.push_str(&format!(
            "<tr class='danger'>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>",
            html_escape(&task.title),
            html_escape(&task.deadline),
            html_escape(&task.status),
        ));
    }

    content.push_str("</table>");
    content.push_str("</div>");

    layout(
        "管理仪表盘",
        "项目经理",
        "项目经理",
        &content,
    )
}

/// 项目列表和新建项目表单
pub fn projects_html(
    store: &Store,
    flash: Option<&str>,
) -> String {
    let mut content = String::new();

    content.push_str(&flash_html(flash));

    content.push_str("<div class='card'>");
    content.push_str("<h3>已有项目</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>名称</th>
            <th>描述</th>
            <th>预算</th>
            <th>开始日期</th>
            <th>截止日期</th>
            <th>状态</th>
        </tr>",
    );

    for project in &store.projects {
        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{:.2}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>",
            html_escape(&project.name),
            html_escape(&project.desc),
            project.budget,
            html_escape(&project.start),
            html_escape(&project.deadline),
            html_escape(&project.status),
        ));
    }

    if store.projects.is_empty() {
        content.push_str(
            "<tr>
                <td colspan='6' class='muted'>暂无项目</td>
            </tr>",
        );
    }

    content.push_str("</table>");
    content.push_str("</div>");

    content.push_str(
        r#"<div class='card'>
<h3>新建项目</h3>

<form method="post" action="/project">
    <p>项目名称：<input name="name" required></p>
    <p>项目描述：<input name="desc"></p>
    <p>预算：<input name="budget" type="number" step="0.01" min="0"></p>
    <p>开始日期：<input name="start" type="date"></p>
    <p>截止日期：<input name="deadline" type="date"></p>
    <button type="submit">创建项目</button>
</form>
</div>"#,
    );

    layout(
        "项目管理",
        "项目经理",
        "项目经理",
        &content,
    )
}

/// 生成任务状态下拉框
fn status_options(current: &str) -> String {
    let statuses = [
        consts::T_TODO,
        consts::T_DOING,
        consts::T_DONE,
    ];

    let mut result = String::new();

    for status in statuses {
        let selected = if status == current {
            " selected"
        } else {
            ""
        };

        result.push_str(&format!(
            "<option value='{}'{}>{}</option>",
            html_escape(status),
            selected,
            html_escape(status),
        ));
    }

    result
}

/// 任务列表、新建任务和状态修改
pub fn tasks_html(
    store: &Store,
    flash: Option<&str>,
) -> String {
    let mut content = String::new();

    content.push_str(&flash_html(flash));

    content.push_str("<div class='card'>");
    content.push_str("<h3>任务列表</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>任务</th>
            <th>项目</th>
            <th>负责人</th>
            <th>优先级</th>
            <th>状态</th>
            <th>预估工时</th>
            <th>截止日期</th>
            <th>操作</th>
        </tr>",
    );

    for task in &store.tasks {
        let project_name = store
            .project_by_id(task.project_id)
            .map(|project| html_escape(&project.name))
            .unwrap_or_else(|| "未知项目".to_string());

        let assignee_name = task
            .assignee
            .and_then(|id| store.member_by_id(id))
            .map(|member| html_escape(&member.name))
            .unwrap_or_else(|| "未分配".to_string());

        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{:.1}</td>
                <td>{}</td>
                <td>
                    <form method='post' action='/task'>
                        <input type='hidden' name='action' value='status'>
                        <input type='hidden' name='task_id' value='{}'>
                        <select name='status'>{}</select>
                        <button type='submit'>更新</button>
                    </form>
                </td>
            </tr>",
            html_escape(&task.title),
            project_name,
            assignee_name,
            html_escape(&task.priority),
            html_escape(&task.status),
            task.estimate_hours,
            html_escape(&task.deadline),
            task.id,
            status_options(&task.status),
        ));
    }

    if store.tasks.is_empty() {
        content.push_str(
            "<tr>
                <td colspan='8' class='muted'>暂无任务</td>
            </tr>",
        );
    }

    content.push_str("</table>");
    content.push_str("</div>");

    content.push_str(
        r#"<div class='card'>
<h3>新建任务</h3>

<form method="post" action="/task">
    <input type="hidden" name="action" value="new">

    <p>项目 ID：
        <input name="project_id" type="number" min="1" required>
    </p>

    <p>任务标题：
        <input name="title" required>
    </p>

    <p>负责人 ID：
        <input name="assignee" type="number" min="1">
    </p>

    <p>优先级：
        <select name="priority">
            <option value="高">高</option>
            <option value="中" selected>中</option>
            <option value="低">低</option>
        </select>
    </p>

    <p>预估工时：
        <input name="estimate_hours" type="number" step="0.5" min="0">
    </p>

    <p>截止日期：
        <input name="deadline" type="date">
    </p>

    <button type="submit">创建任务</button>
</form>
</div>"#,
    );

    layout("任务管理", "成员", "成员", &content)
}

/// 工时登记页面
pub fn timesheet_form_html(
    store: &Store,
    member_id: u32,
) -> String {
    let member = store.member_by_id(member_id);

    let member_name = member
        .map(|item| item.name.as_str())
        .unwrap_or("成员");

    let role = member
        .map(|item| item.role.as_str())
        .unwrap_or("成员");

    let mut content = String::new();

    content.push_str(
        r#"<div class='card'>
<h3>登记工时</h3>

<form method="post" action="/timesheet">
    <p>任务：
        <select name="task_id">"#,
    );

    for task in &store.tasks {
        content.push_str(&format!(
            "<option value='{}'>{}</option>",
            task.id,
            html_escape(&task.title),
        ));
    }

    content.push_str(
        r#"</select>
    </p>

    <p>日期：
        <input name="date" type="date">
    </p>

    <p>工时：
        <input name="hours" type="number" step="0.5" min="0.5" required>
    </p>

    <p>备注：
        <input name="note">
    </p>

    <button type="submit">提交工时</button>
</form>
</div>"#,
    );

    content.push_str(
        "<div class='card'>
        <h3>超预估工时提醒</h3>",
    );

    let mut warning_count = 0;

    for task in &store.tasks {
        let used_hours = store.task_hours(task.id);

        if used_hours > task.estimate_hours {
            warning_count += 1;

            content.push_str(&format!(
                "<p class='danger'>
                    任务《{}》已使用 {:.1} 小时，
                    超过预估 {:.1} 小时。
                </p>",
                html_escape(&task.title),
                used_hours,
                task.estimate_hours,
            ));
        }
    }

    if warning_count == 0 {
        content.push_str(
            "<p class='ok'>
                目前没有超出预估工时的任务。
            </p>",
        );
    }

    content.push_str("</div>");

    layout(
        "工时登记",
        member_name,
        role,
        &content,
    )
}

/// 我的任务和我的工时
pub fn my_page_html(
    store: &Store,
    member_id: u32,
    member_name: &str,
) -> String {
    let mut content = String::new();

    content.push_str("<div class='card'>");
    content.push_str("<h3>我的任务</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>任务</th>
            <th>优先级</th>
            <th>状态</th>
            <th>预估工时</th>
            <th>截止日期</th>
        </tr>",
    );

    let mut task_count = 0;

    for task in &store.tasks {
        if task.assignee != Some(member_id) {
            continue;
        }

        task_count += 1;

        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{:.1}</td>
                <td>{}</td>
            </tr>",
            html_escape(&task.title),
            html_escape(&task.priority),
            html_escape(&task.status),
            task.estimate_hours,
            html_escape(&task.deadline),
        ));
    }

    if task_count == 0 {
        content.push_str(
            "<tr>
                <td colspan='5' class='muted'>
                    暂无分配给你的任务
                </td>
            </tr>",
        );
    }

    content.push_str("</table>");
    content.push_str("</div>");

    content.push_str("<div class='card'>");
    content.push_str("<h3>我的工时</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>任务</th>
            <th>日期</th>
            <th>工时</th>
            <th>备注</th>
        </tr>",
    );

    let mut sheet_count = 0;

    for sheet in &store.timesheets {
        if sheet.member_id != member_id {
            continue;
        }

        sheet_count += 1;

        let task_title = store
            .task_by_id(sheet.task_id)
            .map(|task| html_escape(&task.title))
            .unwrap_or_else(|| "未知任务".to_string());

        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{:.1}</td>
                <td>{}</td>
            </tr>",
            task_title,
            html_escape(&sheet.date),
            sheet.hours,
            html_escape(&sheet.note),
        ));
    }

    if sheet_count == 0 {
        content.push_str(
            "<tr>
                <td colspan='4' class='muted'>
                    暂无工时记录
                </td>
            </tr>",
        );
    }

    content.push_str("</table>");
    content.push_str("</div>");

    layout(
        "我的工作台",
        member_name,
        "成员",
        &content,
    )
}

/// 成员列表和新增成员表单
pub fn members_html(
    store: &Store,
    flash: Option<&str>,
) -> String {
    let mut content = String::new();

    content.push_str(&flash_html(flash));

    content.push_str("<div class='card'>");
    content.push_str("<h3>成员列表</h3>");

    content.push_str(
        "<table>
        <tr>
            <th>ID</th>
            <th>用户名</th>
            <th>姓名</th>
            <th>角色</th>
            <th>费率</th>
        </tr>",
    );

    for member in &store.members {
        content.push_str(&format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{:.2} 元/小时</td>
            </tr>",
            member.id,
            html_escape(&member.username),
            html_escape(&member.name),
            html_escape(member.role.as_str()),
            member.rate,
        ));
    }

    if store.members.is_empty() {
        content.push_str(
            "<tr>
                <td colspan='5' class='muted'>暂无成员</td>
            </tr>",
        );
    }

    content.push_str("</table>");
    content.push_str("</div>");

    content.push_str(
        r#"<div class='card'>
<h3>新增成员</h3>

<form method="post" action="/member">
    <p>用户名：
        <input name="username" required>
    </p>

    <p>密码：
        <input name="password" type="password" required>
    </p>

    <p>姓名：
        <input name="name" required>
    </p>

    <p>角色：
        <select name="role">
            <option value="Pm">项目经理</option>
            <option value="Dev">开发</option>
            <option value="Qa">测试</option>
        </select>
    </p>

    <p>小时费率：
        <input name="rate" type="number" step="0.01" min="0">
    </p>

    <button type="submit">新增成员</button>
</form>
</div>"#,
    );

    layout(
        "成员管理",
        "项目经理",
        "项目经理",
        &content,
    )
}

/// 错误页面
pub fn error_page(message: &str) -> String {
    format!(
        "<h1>出错了</h1>
        <p>{}</p>
        <p><a href='/'>返回首页</a></p>",
        html_escape(message)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_page_contains_login_form() {
        let html = login_page(None);

        assert!(html.contains("action=\"/login\""));
        assert!(html.contains("name=\"username\""));
        assert!(html.contains("name=\"password\""));
    }

    #[test]
    fn login_error_is_escaped() {
        let html = login_page(
            Some("<script>alert(1)</script>")
        );

        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn layout_contains_navigation() {
        let html = layout(
            "测试页面",
            "张三",
            "开发",
            "<p>测试内容</p>",
        );

        assert!(html.contains("张三"));
        assert!(html.contains("开发"));
        assert!(html.contains("/tasks"));
        assert!(html.contains("/my"));
    }
}