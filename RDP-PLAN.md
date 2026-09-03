# RDPlatform —— 研发任务与工时管理平台 · 两天工作安排

> 目标：企业研发团队内部管理工具（核心逻辑 Rust + Web 用户界面）
> 四人并行，延续上一课程项目（kvstore）的分工模式与工程纪律
> 组长：成员A | 仓库：待建（本地 RDPlatform/ 已初始化）

## 〇、评分要点（企业要求，贯穿开发与报告）

1. **系统环境与开发工具**：crate 依赖管理（serde/serde_json/sha2）、分层测试、Git、文档
2. **多学科团队**：PM/Dev/QA 三角色 + 权限分级 + 各自工作流（页面演示角色视角）
3. **工程管理与经济决策**：费率成本核算、预算超支预警、估算偏差、成员负载、逾期提醒

## 一、技术路线

| 项 | 方案 |
|---|---|
| 语言 | Rust（edition 2024） |
| 界面 | 纯 Rust 手写 HTTP（http.rs 已完整实现）+ HTML 页面 |
| 持久化 | JSON 文件 data/db.json（serde + serde_json） |
| 认证 | sha2 加盐哈希 + token 会话 + Cookie（auth.rs 基线可用） |
| 测试 | cargo test 分层覆盖 |

**依赖**：`serde` `serde_json` `sha2`（首次 `cargo build` 联网下载）
> 若 crates.io 下载失败：配置镜像（如 `rsproxy.cn`）：
> `C:\Users\<你>\.cargo\config.toml` 写入 `[source.crates-io] replace-with='rsproxy'` 等。

## 二、文件分工（每人只改自己的文件！）

```
RDPlatform/
├── src/
│   ├── main.rs   # A：入口（完整）
│   ├── lib.rs    # A：模块声明（完整）
│   ├── http.rs   # A：HTTP 协议层（完整，勿改）
│   ├── api.rs    # A：路由分发（登录/登出/鉴权完整；写操作接线）
│   ├── model.rs  # B：数据模型（完整，字段勿改）
│   ├── store.rs  # B：JSON 持久化 + CRUD + 5 项统计（**待 B 实现**）
│   ├── auth.rs   # D：认证会话（基线可用；**待 D 增强+测试**）
│   └── page.rs   # C：HTML 页面（**待 C 实现**）
└── tests/        # D：单元/集成/HTTP 流程测试（**待 D**）
```

| 成员 | 文件 | 任务 | 今日交付点 |
|---|---|---|---|
| **A** | http.rs / api.rs / main.rs / lib.rs | 已交付骨架+登录鉴权完整；集成协调、文档 | 集成节点主持、README/DEMO |
| **B** | model.rs / store.rs | model 完成✅；**实现 store CRUD、5 项统计、seed 复核** | 16:30①前：CRUD+find_member_by_username（登录依赖） |
| **C** | page.rs | **实现全部页面**：layout→登录页✅→仪表盘/项目/任务/工时/我的/成员/首页 | 16:30①：layout+登录页跑通；②前：全部页面 |
| **D** | auth.rs / tests/ | auth 基线✅；**补强（登录失败限制/会话过期可选）+ 测试**：auth 单测、store 统计单测、HTTP 登录/权限集成测试 | 16:30①：auth 测试绿；②前：全量测试绿 |

## 三、接口契约（防冲突，勿改签名）

### 路由表（A 已按此实现 dispatch）
| 路由 | 方法 | 页面/动作 | 权限 |
|---|---|---|---|
| /login | GET/POST | 登录页/校验 | 公开 |
| /logout | GET | 登出 | 已登录 |
| / | GET | 首页（角色快捷入口） | 已登录 |
| /dashboard | GET | 管理仪表盘（5 项统计） | PM |
| /projects | GET | 项目列表+新建表单 | PM |
| /project | POST | 创建项目 | PM |
| /members | GET | 成员列表+新增表单 | PM |
| /member | POST | 新增成员 | PM |
| /tasks | GET | 任务列表（含新建/状态流转） | 全员 |
| /task | POST | action=new 新建；action=status 流转 | 全员 |
| /timesheet | GET/POST | 工时登记（超预估预警） | Dev/QA |
| /my | GET | 我的任务+我的工时 | 全员 |

### Store 统计函数（B 实现，C 页面调用）
```rust
pub fn project_progress(&self, pid: u32) -> (u32, u32)          // (完成数, 总数)
pub fn project_cost(&self, pid: u32) -> f64                     // 实际成本=Σ工时×费率
pub fn estimate_deviation(&self, pid: u32) -> f64               // 实际-预估(小时)
pub fn member_load(&self) -> Vec<(u32, String, f64)>            // (id,姓名,累计工时)
pub fn overdue_tasks(&self, today: &str) -> Vec<Task>           // 逾期未完成任务
pub fn task_hours(&self, task_id: u32) -> f64                   // 任务累计工时（超预估预警用）
pub fn today_iso() -> String                                    // 已实现
```

### 演示账号（seed 已写入）
pm1/pm123（项目经理）dev1/dv123（开发）qa1/qa123（测试）

### 页面表单字段契约（C 必读——与 api.rs 的 req.param() 严格一致）

| 表单 action | 字段（name=） | 说明 |
|---|---|---|
| POST /project | `name` `desc` `budget` `start` `deadline` | 新建项目；name 必填 |
| POST /member | `username` `password` `name` `role` `rate` | 新增成员；role 取值 Pm/Dev/Qa |
| POST /task?action=new | `project_id` `title` `assignee` `priority` `estimate_hours` `deadline` | 新建任务；title 必填；priority 取 高/中/低 |
| POST /task?action=status | `task_id` `status` | 流转状态；status 取 待办/进行/完成 |
| POST /timesheet | `task_id` `date` `hours` `note` | 登记工时；hours>0 |

**C 页面调用约定**：
- 页面函数只返回 HTML 主体，api.rs 已统一 http_response 包装与 save；
- 数据来源直接调 store 方法（统计见上节；列表用 `store.members/projects/tasks/timesheets` 字段 + `task_by_id/member_by_id` 关联显示）；
- 超预估预警：`store.task_hours(task_id) > task.estimate_hours` 时在工时登记页/任务页提示；
- 逾期标红：仪表盘调 `store.overdue_tasks(store.today_iso())`；
- 所有用户数据渲染前必须 `http::html_escape`。

### D 测试约定
- auth 单测（已有 3 个基线）+ HTTP 登录流程集成测试：
  POST /login → 302 + Set-Cookie → 带 Cookie GET / → 200；
  dev1 访问 /dashboard → 403；
- store 统计单测已由 B 提供 9 个，D 复核即可；
- tests/ 文件命名：`login_tests.rs`（D）。


## 四、两天进度
```
Day1
 09:00-09:30 契约会：A 宣读本文档（路由/字段/统计签名），全员确认
 09:30-12:00 并行：
   A 已交骨架 → 协助联调 / 补 api 细节
   B store CRUD+find_member_by_username+统计函数
   C page: layout + 登录页 + dashboard 雏形
   D auth 单测 + store 统计单测 + 登录集成测试骨架
 13:30-16:00 并行：
   B 统计函数完成 + seed 复核 + 数据测试
   C 项目/任务/工时/我的页面
   A api 各写操作联调（create_project 等）+ 路由 404/403 完善
   D HTTP 登录流程测试（登录→Cookie→访问 /）
 16:30 集成节点①：合并→cargo build→登录跑通（pm1 登录跳首页）→cargo test→git tag day1
Day2
 09:00-12:00：
   C 完成全部页面（仪表盘 5 统计展示、超预估预警、逾期标红）
   B 统计数据正确性复核 + 补边界
   A 全路由联调（三角色视角走查）+ README
   D 权限测试（dev 访问 /dashboard 得 403）+ 全量测试
 13:30-16:00 收尾：
   端到端演示走查：登录→建项目→派任务→登工时→仪表盘（成本/预算/负载/逾期）
   cargo test 全绿 → git tag day2 → 验收记录 + DEMO
```

## 五、验收清单（对照）

1. 登录：pm1/dev1/qa1 三种账号可登录，错误密码提示且不进入系统
2. 权限：dev1 访问 /dashboard、/projects 返回 403（"仅项目经理可用"）
3. PM：建项目（预算/起止日期）、新增成员（角色/费率）、派任务
4. Dev/QA：工时登记；累计工时超预估给出预警
5. 仪表盘（PM）：项目进度、实际成本 vs 预算（超 80% 黄/100% 红）、估算偏差、成员负载、逾期任务标红
6. 我的视图：只显示自己的任务与工时
7. 持久化：重启后数据不丢（data/db.json）
8. `cargo test` 全绿

## 六、纪律（延续上项目教训）

- 每人只改自己的文件；接口变更必须先群里声明，由组长确认
- 表单字段名以路由表/示例为准，C 页面与 A 的 req.param() 保持一致
- 写操作后必须 store.save()（api.rs 已统一处理，勿在页面层绕过）
- 代码合并当天跑全量测试；遇到编译错当场解决
