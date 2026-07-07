# 链路追踪

> Feature: `trace`（自动启用 `log` + `afast/hook` + `afast-ordinary-http` + `sse`）
>
> 可选 Feature：`trace-sqlite` / `trace-http` / `trace-tcp` / `trace-receive-http` / `trace-receive-tcp`

基于 afast hook 系统实现的本地链路追踪，自动为每个请求创建 Span，提供 Web UI 查询和实时推送。支持多种存储后端，支持跨项目上报。

## 存储后端

| Feature | 说明 | 方向 | 配置 |
|---------|------|------|------|
| `trace-sqlite` | 本地 SQLite 存储（默认） | 本地 | `[tracing]` `db_path` |
| `trace-http` | HTTP 远程存储（afastdata 二进制协议） | 发送 | `[tracing.http]` |
| `trace-tcp` | TCP 远程存储（afastdata 序列化） | 发送 | `[tracing.tcp]` |
| `trace-receive-http` | 启用 HTTP 接收（`afast-http` 传输） | 接收 | 无需额外配置 |
| `trace-receive-tcp` | 启用 TCP 接收（`afast-tcp` 传输） | 接收 | 无需额外配置 |

### 优先级

1. 用户通过 `set_trace_store()` 注册的自定义存储
2. 配置文件 `[tracing].db_path` → SQLite
3. 配置文件 `[tracing.http]` → HTTP
4. 配置文件 `[tracing.tcp]` → TCP
5. 都没有 → 空存储（不持久化）

## 配置

### SQLite（默认）

```toml
[tracing]
service_name = "afaster"
db_path = "tracing.db"
# url = "/tracing"
# retention_days = 7
```

### HTTP

```toml
[tracing]
service_name = "afaster"

[tracing.http]
url = "http://collector:8080"
path = "/tracing/report"   # 默认值
token = "my-secret"        # 可选，Bearer Token
timeout = 5                 # 可选，请求超时（秒）
```

### TCP

```toml
[tracing]
service_name = "afaster"

[tracing.tcp]
addr = "collector:9090"
connect_timeout = 5           # 可选，连接超时（秒）
reconnect_interval = 3        # 可选，重连间隔（秒）
```

## 跨项目上报

其他 afaster 项目可以将 span 数据上报到本项目。只需在发送方配置 `[tracing.http]` 或 `[tracing.tcp]` 指向本项目的地址，并在本项目启用对应的接收 feature。

| 接收 Feature | 传输协议 | 依赖的 afast 传输 |
|-------------|---------|------------------|
| `trace-receive-http` | HTTP binary | `afast-http` |
| `trace-receive-tcp` | TCP binary | `afast-tcp` |

接收端通过已注册的 `report` / `report_batch` binary handler 接收数据，无需额外配置。发送端和接收端使用相同的 afastdata 二进制协议。

### 示例：A 项目上报到 B 项目

**A 项目（发送方）config.toml：**

```toml
[tracing]
service_name = "service-a"
db_path = "tracing.db"      # 本地也存储一份

[tracing.http]
url = "http://service-b:8080"
path = "/tracing/report"
```

**B 项目（接收方）config.toml：**

```toml
[tracing]
service_name = "service-b"
db_path = "tracing.db"
```

**B 项目 Cargo.toml：**

```toml
[dependencies]
afaster = { version = "0.0.6", features = ["trace-sqlite", "trace-receive-http"] }
```

## 使用

```rust
use afaster::AFaster;

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .run()
        .await;
}
```

启用后自动生效：
- Hook 自动采集每个请求的 span
- 注册 binary handler（上报/查询/统计）
- 注册 ordinary-http 页面路由（Web UI）
- 注册 SSE 实时推送端点

## 工作原理

```
请求到达
  │
  ├─ before_request / on_connect（Hook）
  │    ├─ 检查 no_trace 属性 → 跳过则不追踪
  │    ├─ 生成 trace_id / span_id
  │    ├─ 写入 TraceContext 到 ctx（handler 可通过 Ctx<TraceContext> 提取）
  │    └─ 返回 RequestGuard / ConnectionGuard
  │
  ├─ Handler 执行（可使用 span() / span_root() 创建子 span）
  │
  ├─ 成功 → on_response → 写入 span 到 channel
  │
  └─ 失败 → on_error → 写入 span（含错误信息）到 channel

后台任务
  │
  ├─ 批量消费 channel → 写入存储
  │
  └─ 广播 SSE 事件 → 前端实时更新
```

## TraceContext

Hook 自动创建 `TraceContext` 并写入 `ctx.ctx`，handler 通过 `Ctx<TraceContext>` 提取。

```rust
#[derive(Clone)]
pub struct TraceContext {
    pub trace_id: String,       // 当前 trace ID
    pub span_id: String,        // 当前 span ID（父级）
    pub service_name: String,   // 服务名称
    pub transport: String,      // 传输协议：http / ws / tcp / sse
    pub handler_name: String,   // 当前 handler 名称
    pub handler_desc: String,   // 当前 handler 描述
}
```

## 子 span API

### Guard 模式（推荐用于顺序执行的代码块）

| 方法 | 参数 | 说明 |
|------|------|------|
| `span(ctx)` | ctx | 自动继承 handler 名称和描述，`is_root=false` |
| `span_name(ctx, name)` | ctx + name | 指定名称，描述为空，`is_root=false` |
| `span_with(ctx, name, desc)` | ctx + name + desc | 指定名称和描述，`is_root=false` |
| `span_root(ctx)` | ctx | 同 `span()`，但 `is_root=true`（列表独立显示） |
| `span_root_name(ctx, name)` | ctx + name | 同 `span_name()`，但 `is_root=true` |
| `span_root_with(ctx, name, desc)` | ctx + name + desc | 同 `span_with()`，但 `is_root=true` |

```rust
#[handler(desc("用户登录"))]
async fn login(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(req): Data<LoginReq>,
) -> afast::Result<LoginResp> {
    // 自动继承 handler 名称 "login" 和描述 "用户登录"
    let _s = state.tracing.span(&trace);
    sqlx::query_as(...).fetch_one(&pool).await?;

    // 自定义名称，描述为空
    let _s = state.tracing.span_name(&trace, "hash_password");

    // 自定义名称和描述
    let _s = state.tracing.span_with(&trace, "db_query", "查询用户表");

    // root 模式：列表中独立显示（适合长连接消息）
    let _s = state.tracing.span_root_with(&trace, "message", "处理消息");
}
```

### 闭包模式（推荐用于单行表达式）

| 方法 | 参数 | 说明 |
|------|------|------|
| `trace(ctx, f)` | ctx + 闭包 | 自动继承 handler 名称和描述 |
| `trace_name(ctx, name, f)` | ctx + name + 闭包 | 指定名称，描述为空 |
| `trace_with(ctx, name, desc, f)` | ctx + name + desc + 闭包 | 指定名称和描述 |
| `trace_root(ctx, f)` | ctx + 闭包 | 同 `trace()`，但 `is_root=true` |
| `trace_root_name(ctx, name, f)` | ctx + name + 闭包 | 同 `trace_name()`，但 `is_root=true` |
| `trace_root_with(ctx, name, desc, f)` | ctx + name + desc + 闭包 | 同 `trace_with()`，但 `is_root=true` |

```rust
// 自动继承 handler 信息
let user = state.tracing.trace(&trace, || {
    sqlx::query_as(...).fetch_one(&pool)
}).await?;

// 自定义名称
let data = state.tracing.trace_name(&trace, "process", || {
    heavy_computation(input)
}).await;

// 自定义名称和描述
let result = state.tracing.trace_with(&trace, "db_write", "写入数据库", || {
    sqlx::query(...).execute(&pool)
}).await?;
```

### Guard vs 闭包的区别

| 特性 | Guard 模式 | 闭包模式 |
|------|-----------|---------|
| 语法 | `let _s = state.tracing.span(&ctx);` | `state.tracing.trace(&ctx, \|\| { ... }).await` |
| 生命周期 | `_s` 离开作用域时上报 | 闭包返回时上报 |
| 适用场景 | 多行代码块、需要提前 drop | 单行表达式、需要返回值 |
| 错误处理 | 需手动 `set_error()` | 自动继承闭包结果 |

### Root vs 非 Root

| 类型 | 列表显示 | 详情显示 | 适用场景 |
|------|---------|---------|---------|
| `span()` / `trace()` | ❌ 不显示 | ✅ 作为子项 | 普通子操作 |
| `span_root()` / `trace_root()` | ✅ 独立显示 | ✅ 作为子项 | 长连接消息、需要独立追踪的操作 |

## 长连接支持

长连接（WS/TCP/SSE）通过 `on_connect` / `on_disconnect` 追踪连接生命周期：

```
连接建立 → on_connect 创建连接 span
  ├─ 消息1 → span_root() 创建子 span（列表独立显示）
  ├─ 消息2 → span_root() 创建子 span
  └─ 消息N → span_root() 创建子 span
连接断开 → on_disconnect 上报连接总时长
```

```rust
#[afast::ws(desc("WebSocket 聊天"))]
async fn ws_chat(
    State(state): State<AppState>,
    Ctx(trace): Ctx<TraceContext>,
    sender: WsSender,
    mut receiver: WsReceiver,
) -> afast::Result<()> {
    loop {
        match receiver.recv().await {
            Some(WsMessage::Text(text)) => {
                // 每条消息独立显示在列表中
                let _s = state.tracing.span_root_with(&trace, "ws_message", "处理消息");
                process_message(text).await;
            }
            _ => break,
        }
    }
    Ok(())
}
```

## no_trace 属性

通过 `#[handler(..., no_trace)]` 或 `#[afast::sse(..., no_trace)]` 排除不需要追踪的接口：

```rust
#[handler(desc("健康检查"), no_trace)]
async fn health() -> afast::Result<()> { Ok(()) }

#[afast::sse(desc("SSE 推送"), no_trace)]
async fn sse_handler(sender: SseSender) -> afast::Result<()> { Ok(()) }
```

## Web UI

访问配置的 `url`（默认 `/tracing`）即可打开 Web UI：

- **统计卡片**：总请求数、错误数、错误率、平均耗时、P50、P99
- **筛选**：按状态、协议（Call/WS/Long/SSE）、Handler 名称、最小耗时过滤
- **列表**：显示所有 `is_root=true` 的 span，支持分页
- **详情弹窗**：嵌套矩形树图 + 层级列表
- **实时推送**：SSE 自动推送新 trace，支持开关切换
- **主题切换**：暗色/亮色主题

## 存储扩展

通过实现 `TraceStore` trait 可自定义存储后端：

```rust
use afaster::trace::store::{TraceStore, SpanData, TraceListResult, ChildrenResult, TraceStats};
use std::future::Future;
use std::pin::Pin;

struct MyStore { /* ... */ }

impl TraceStore for MyStore {
    fn insert_span(&self, span: SpanData) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async { /* ... */ })
    }
    fn insert_spans(&self, spans: Vec<SpanData>) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async { /* ... */ })
    }
    // ... 其他方法
}
```

注册自定义存储：

```rust
AFaster::new("config.toml".to_string()).await
    .unwrap()
    .set_trace_store(my_store)
    .run()
    .await;
```

## 统计说明

- **P50/P95/P99**：仅统计非长连接请求（`long_connection=false`）且 `duration_us > 0` 的数据
- **错误率**：统计所有 trace（含长连接）
- **0ms 数据**：不参与耗时统计，列表中显示为 `0ms`

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 52801 | SQLite init failed | SQLite 初始化失败 |
| 52802 | SQLite error | SQLite 操作失败 |
| 52803 | Store not configured | 未配置存储后端 |
| 52804 | HTTP init failed | HTTP 存储初始化失败 |
| 52805 | HTTP request failed | HTTP 请求失败 |
| 52806 | TCP init failed | TCP 存储初始化失败 |
| 42801 | Invalid span data | Span 数据格式错误 |
| 42802 | Trace not found | Trace 不存在 |
