//! 本地链路追踪模块
//!
//! 自动拦截每个请求创建 Span，存储到 SQLite，提供查询接口和 Web UI。
//!
//! # Feature
//!
//! 启用 `tracing-local` 后自动生效：
//! - Hook 自动采集每个请求的 span
//! - 注册 binary handler（上报/查询/统计）
//! - 注册 ordinary-http 页面路由（Web UI）
//!
//! # 配置
//!
//! ```toml
//! [tracing]
//! service_name = "my-service"
//! db_path = "tracing.db"
//! # url = "/tracing"
//! # retention_days = 7
//! ```

pub mod err;
pub mod handler;
#[cfg(feature = "trace-http")]
pub mod http;
pub mod page;
#[cfg(feature = "trace-sqlite")]
pub mod sqlite;
pub mod store;
#[cfg(feature = "trace-tcp")]
pub mod tcp;

use serde::Deserialize;
use tokio::sync::mpsc;

#[cfg(feature = "trace-sqlite")]
use self::sqlite::SqliteTraceStore;
pub use self::store::{EmptyTraceStore, TraceStore};
use self::store::{SpanData, SpanStatus};

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

/// 链路追踪配置
#[derive(Clone, Deserialize)]
pub struct TracingConfig {
    /// 服务名称
    #[serde(default = "default_service_name")]
    pub service_name: String,
    /// SQLite 数据库文件路径（可选，需要 `trace-sqlite` feature）
    #[cfg(feature = "trace-sqlite")]
    pub db_path: Option<String>,
    /// HTTP 存储配置（可选，需要 `trace-http` feature，发送端）
    #[cfg(feature = "trace-http")]
    pub http: Option<http::HttpTraceStoreConfig>,
    /// TCP 存储配置（可选，需要 `trace-tcp` feature，发送端）
    #[cfg(feature = "trace-tcp")]
    pub tcp: Option<tcp::TcpTraceStoreConfig>,
    /// Web UI 路径
    #[serde(default = "default_url")]
    pub url: String,
    /// Web UI SSE 路径
    #[serde(default = "default_event")]
    pub event: String,
    /// 数据保留天数，0 表示不清理
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_service_name() -> String {
    "afaster".to_string()
}
fn default_url() -> String {
    "/tracing".to_string()
}
fn default_event() -> String {
    "/tracing/events".to_string()
}
fn default_retention_days() -> u32 {
    7
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            #[cfg(feature = "trace-sqlite")]
            db_path: None,
            #[cfg(feature = "trace-http")]
            http: None,
            #[cfg(feature = "trace-tcp")]
            tcp: None,
            url: default_url(),
            event: default_event(),
            retention_days: default_retention_days(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TraceContext：per-request 链路上下文
// ═══════════════════════════════════════════════════════════════

/// 请求级链路上下文，由 Hook 自动写入 `ctx.ctx`，handler 通过 `Ctx<TraceContext>` 提取。
///
/// 用于在 handler 内部创建子 span，形成多层 trace 链路。
#[derive(Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub service_name: String,
    pub transport: String,
    pub handler_name: String,
    pub handler_desc: String,
}

// ═══════════════════════════════════════════════════════════════
//  TracingService
// ═══════════════════════════════════════════════════════════════

/// SSE 推送的 trace 摘要
#[derive(Clone, serde::Serialize)]
pub struct TraceEvent {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub handler_name: String,
    pub handler_desc: String,
    pub service_name: String,
    pub transport: String,
    pub is_binary: bool,
    pub method: String,
    pub long_connection: bool,
    pub is_root: bool,
    pub start_time: i64,
    pub duration_us: i64,
    pub span_count: i64,
    pub has_error: bool,
}

/// 链路追踪服务
///
/// 持有存储和 channel sender，用于 Hook 向后台 task 发送 span。
/// 内部使用 `Box<dyn TraceStore>` 以支持动态存储后端。
#[derive(Clone)]
pub struct TracingService {
    pub store: std::sync::Arc<dyn TraceStore>,
    tx: mpsc::Sender<SpanData>,
    broadcast: tokio::sync::broadcast::Sender<TraceEvent>,
    pub config: TracingConfig,
}

impl TracingService {
    /// 获取存储引用
    pub fn store(&self) -> &dyn TraceStore {
        self.store.as_ref()
    }

    /// 获取配置引用
    pub fn config(&self) -> &TracingConfig {
        &self.config
    }

    /// 订阅实时 trace 事件
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TraceEvent> {
        self.broadcast.subscribe()
    }

    pub async fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let config: TracingConfig = crate::state::extract(table, "tracing")?;
        init_tracing(&config).await
    }

    /// 上报子 span（handler 内部调用，用于构建多层 trace 链路）
    pub async fn report_child_span(
        &self,
        trace_id: &str,
        parent_span_id: &str,
        handler_name: &str,
        handler_desc: &str,
        start_ms: i64,
        duration_us: i64,
        status: SpanStatus,
        error_code: Option<i32>,
        error_message: Option<String>,
    ) {
        let span = SpanData {
            trace_id: trace_id.to_string(),
            span_id: generate_hex_id(16),
            parent_span_id: Some(parent_span_id.to_string()),
            handler_name: handler_name.to_string(),
            handler_desc: handler_desc.to_string(),
            transport: self.config.service_name.clone(), // fallback
            is_binary: false,
            method: String::new(),
            long_connection: false,
            is_root: false,
            start_time: start_ms,
            duration_us: duration_us.max(1),
            status,
            error_code,
            error_message,
            service_name: self.config.service_name.clone(),
        };
        let _ = self.tx.try_send(span);
    }

    /// 创建子 span guard（自动继承 TraceContext 的 handler 信息）。
    ///
    /// # 用法
    /// ```ignore
    /// let _s = state.tracing.span(&trace);
    /// let result = sqlx::query_as(...).fetch_one(pool).await?;
    /// ```
    pub fn span(&self, trace: &TraceContext) -> SpanGuard {
        SpanGuard {
            trace_id: trace.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: trace.span_id.clone(),
            handler_name: trace.handler_name.clone(),
            handler_desc: trace.handler_desc.clone(),
            transport: trace.transport.clone(),
            service_name: self.config.service_name.clone(),
            is_root: false,
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建子 span guard（仅传 name，desc 为空）。
    pub fn span_name(&self, trace: &TraceContext, name: &str) -> SpanGuard {
        SpanGuard {
            trace_id: trace.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: trace.span_id.clone(),
            handler_name: name.to_string(),
            handler_desc: String::new(),
            transport: trace.transport.clone(),
            service_name: self.config.service_name.clone(),
            is_root: false,
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建子 span guard（传 name 和 desc）。
    pub fn span_with(&self, trace: &TraceContext, name: &str, desc: &str) -> SpanGuard {
        SpanGuard {
            trace_id: trace.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: trace.span_id.clone(),
            handler_name: name.to_string(),
            handler_desc: desc.to_string(),
            transport: trace.transport.clone(),
            service_name: self.config.service_name.clone(),
            is_root: false,
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建子 span guard（root 模式，自动继承 handler 信息）。
    pub fn span_root(&self, trace: &TraceContext) -> SpanGuard {
        SpanGuard {
            trace_id: trace.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: trace.span_id.clone(),
            handler_name: trace.handler_name.clone(),
            handler_desc: trace.handler_desc.clone(),
            transport: trace.transport.clone(),
            service_name: self.config.service_name.clone(),
            is_root: true,
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建子 span guard（root 模式，仅传 name）。
    pub fn span_root_name(&self, trace: &TraceContext, name: &str) -> SpanGuard {
        SpanGuard {
            trace_id: trace.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: trace.span_id.clone(),
            handler_name: name.to_string(),
            handler_desc: String::new(),
            transport: trace.transport.clone(),
            service_name: self.config.service_name.clone(),
            is_root: true,
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
            error_code: None,
            error_message: None,
        }
    }

    /// 创建子 span guard（root 模式，传 name 和 desc）。
    pub fn span_root_with(&self, trace: &TraceContext, name: &str, desc: &str) -> SpanGuard {
        SpanGuard {
            trace_id: trace.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: trace.span_id.clone(),
            handler_name: name.to_string(),
            handler_desc: desc.to_string(),
            transport: trace.transport.clone(),
            service_name: self.config.service_name.clone(),
            is_root: true,
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
            error_code: None,
            error_message: None,
        }
    }

    // ── 闭包模式 ──

    /// 包裹异步闭包，自动继承 handler 信息（闭包模式）。
    pub async fn trace<F, Fut, T>(&self, ctx: &TraceContext, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        self.trace_with(ctx, &ctx.handler_name, &ctx.handler_desc, f)
            .await
    }

    /// 包裹异步闭包，仅传 name（闭包模式）。
    pub async fn trace_name<F, Fut, T>(&self, ctx: &TraceContext, name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        self.trace_with(ctx, name, "", f).await
    }

    /// 包裹异步闭包，传 name 和 desc（闭包模式）。
    pub async fn trace_with<F, Fut, T>(
        &self,
        ctx: &TraceContext,
        handler_name: &str,
        handler_desc: &str,
        f: F,
    ) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        let span = SpanData {
            trace_id: ctx.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: Some(ctx.span_id.clone()),
            handler_name: handler_name.to_string(),
            handler_desc: handler_desc.to_string(),
            transport: ctx.transport.clone(),
            is_binary: false,
            method: String::new(),
            long_connection: false,
            is_root: false,
            start_time: chrono::Utc::now().timestamp_micros() - duration.as_micros() as i64,
            duration_us: duration.as_micros() as i64,
            status: SpanStatus::Ok,
            error_code: None,
            error_message: None,
            service_name: self.config.service_name.clone(),
        };
        let _ = self.tx.try_send(span);
        result
    }

    /// 包裹异步闭包，自动继承 handler 信息（闭包 + root 模式）。
    pub async fn trace_root<F, Fut, T>(&self, ctx: &TraceContext, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        self.trace_root_with(ctx, &ctx.handler_name, &ctx.handler_desc, f)
            .await
    }

    /// 包裹异步闭包，仅传 name（闭包 + root 模式）。
    pub async fn trace_root_name<F, Fut, T>(&self, ctx: &TraceContext, name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        self.trace_root_with(ctx, name, "", f).await
    }

    /// 包裹异步闭包，传 name 和 desc（闭包 + root 模式）。
    pub async fn trace_root_with<F, Fut, T>(
        &self,
        ctx: &TraceContext,
        handler_name: &str,
        handler_desc: &str,
        f: F,
    ) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        let span = SpanData {
            trace_id: ctx.trace_id.clone(),
            span_id: generate_hex_id(16),
            parent_span_id: Some(ctx.span_id.clone()),
            handler_name: handler_name.to_string(),
            handler_desc: handler_desc.to_string(),
            transport: ctx.transport.clone(),
            is_binary: false,
            method: String::new(),
            long_connection: false,
            is_root: true,
            start_time: chrono::Utc::now().timestamp_micros() - duration.as_micros() as i64,
            duration_us: duration.as_micros() as i64,
            status: SpanStatus::Ok,
            error_code: None,
            error_message: None,
            service_name: self.config.service_name.clone(),
        };
        let _ = self.tx.try_send(span);
        result
    }
}

/// 子 span guard，实现 RAII：drop 时自动上报 span。
///
/// 可作为子 span 的父级，实现嵌套：
/// ```ignore
/// let s1 = state.tracing.span(&trace, "fetch_data", "获取数据");
/// {
///     let _s2 = state.tracing.span(&s1, "query_db", "查询数据库");
///     // ...
/// }
/// // s1 drop 时自动上报
/// ```
pub struct SpanGuard {
    trace_id: String,
    span_id: String,
    parent_span_id: String,
    handler_name: String,
    handler_desc: String,
    transport: String,
    service_name: String,
    is_root: bool,
    start: std::time::Instant,
    tx: mpsc::Sender<SpanData>,
    error_code: Option<i32>,
    error_message: Option<String>,
}

impl SpanGuard {
    /// 标记此 span 为错误状态
    pub fn set_error(&mut self, code: i32, message: impl Into<String>) {
        self.error_code = Some(code);
        self.error_message = Some(message.into());
    }

    /// 以此 guard 为父级，生成子 TraceContext。
    /// 传给 `state.tracing.span(&child_ctx, ...)` 即可创建嵌套 span。
    pub fn child_ctx(&self) -> TraceContext {
        TraceContext {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            service_name: self.service_name.clone(),
            transport: self.transport.clone(),
            handler_name: self.handler_name.clone(),
            handler_desc: self.handler_desc.clone(),
        }
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        let span = SpanData {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: Some(self.parent_span_id.clone()),
            handler_name: self.handler_name.clone(),
            handler_desc: self.handler_desc.clone(),
            transport: self.transport.clone(),
            is_binary: false,
            method: String::new(),
            long_connection: false,
            is_root: self.is_root,
            start_time: chrono::Utc::now().timestamp_micros() - duration.as_micros() as i64,
            duration_us: duration.as_micros() as i64,
            status: if self.error_code.is_some() {
                SpanStatus::Error
            } else {
                SpanStatus::Ok
            },
            error_code: self.error_code,
            error_message: self.error_message.clone(),
            service_name: self.service_name.clone(),
        };
        #[cfg(feature = "log")]
        if self.error_code.is_some() {
            ::tracing::warn!(
                trace_id = %self.trace_id,
                span_id = %self.span_id,
                handler = %self.handler_name,
                duration_us = duration.as_micros() as i64,
                error_code = self.error_code,
                error_message = self.error_message.as_deref().unwrap_or(""),
                "trace: span [ERROR]"
            );
        } else {
            ::tracing::info!(
                trace_id = %self.trace_id,
                span_id = %self.span_id,
                handler = %self.handler_name,
                transport = %self.transport,
                is_root = self.is_root,
                duration_us = duration.as_micros() as i64,
                "trace: span"
            );
        }
        let _ = self.tx.try_send(span);
    }
}

// ═══════════════════════════════════════════════════════════════

/// 初始化链路追踪
///
/// 自动检测配置中的存储后端：
/// 1. 如果用户通过 `set_trace_store` 注册了自定义存储，使用它
/// 2. 如果配置了 `db_path`，使用 SQLite（需要 `trace-sqlite` feature）
/// 3. 如果配置了 `http`，使用 HTTP（需要 `trace-http` feature）
/// 4. 如果配置了 `tcp`，使用 TCP（需要 `trace-tcp` feature）
/// 5. 如果都没有，使用空存储
pub async fn init_tracing(config: &TracingConfig) -> crate::Result<TracingService> {
    // 优先级：db_path > http > tcp
    #[cfg(feature = "trace-sqlite")]
    if let Some(ref db_path) = config.db_path {
        return init_tracing_with_store(config, SqliteTraceStore::new(db_path).await?).await;
    }

    #[cfg(feature = "trace-http")]
    if let Some(ref http_config) = config.http {
        return init_tracing_with_store(config, http::HttpTraceStore::new(http_config.clone())?)
            .await;
    }

    #[cfg(feature = "trace-tcp")]
    if let Some(ref tcp_config) = config.tcp {
        return init_tracing_with_store(config, tcp::TcpTraceStore::new(tcp_config.clone())?).await;
    }

    // 没有配置任何存储，使用空存储
    init_tracing_with_store(config, EmptyTraceStore).await
}

/// 使用自定义存储初始化链路追踪
///
/// 用户实现 `TraceStore` trait 后传入即可替换存储后端。
///
/// # 示例
/// ```ignore
/// let my_store = MyCustomStore::new().await?;
/// let service = afaster::trace::init_tracing_with_store(&config, my_store).await?;
/// ```
pub async fn init_tracing_with_store(
    config: &TracingConfig,
    store: impl TraceStore,
) -> crate::Result<TracingService> {
    let (tx, mut rx) = mpsc::channel::<SpanData>(4096);
    let (broadcast, _) = tokio::sync::broadcast::channel::<TraceEvent>(256);

    let store: std::sync::Arc<dyn TraceStore> = std::sync::Arc::new(store);

    // 后台 task：批量消费 span 写入存储 + 广播新 trace
    let store_clone = store.clone();
    let broadcast_clone = broadcast.clone();
    tokio::spawn(async move {
        let mut batch = Vec::with_capacity(64);
        loop {
            let deadline = tokio::time::sleep(std::time::Duration::from_millis(100));
            tokio::pin!(deadline);

            loop {
                tokio::select! {
                    Some(span) = rx.recv() => {
                        batch.push(span);
                        if batch.len() >= 64 {
                            break;
                        }
                    }
                    _ = &mut deadline => {
                        break;
                    }
                    else => {
                        if !batch.is_empty() {
                            let _ = store_clone.insert_spans(batch.clone()).await;
                            batch.clear();
                        }
                        return;
                    }
                }
            }

            if !batch.is_empty() {
                if let Err(e) = store_clone.insert_spans(batch.clone()).await {
                    #[cfg(feature = "log")]
                    ::tracing::error!("tracing batch insert error: {}", e);
                }

                // 广播所有 is_root=true 的 span（每个 span 独立事件）
                for span in &batch {
                    if span.is_root {
                        #[cfg(feature = "log")]
                        ::tracing::trace!(
                            trace_id = %span.trace_id,
                            handler = span.handler_name,
                            "trace: sse broadcast"
                        );
                        let _ = broadcast_clone.send(TraceEvent {
                            trace_id: span.trace_id.clone(),
                            span_id: span.span_id.clone(),
                            parent_span_id: span.parent_span_id.clone(),
                            handler_name: span.handler_name.clone(),
                            handler_desc: span.handler_desc.clone(),
                            service_name: span.service_name.clone(),
                            transport: span.transport.clone(),
                            is_binary: span.is_binary,
                            method: span.method.clone(),
                            long_connection: span.long_connection,
                            is_root: span.is_root,
                            start_time: span.start_time,
                            duration_us: span.duration_us,
                            span_count: 1,
                            has_error: span.status == SpanStatus::Error,
                        });
                    }
                }

                batch.clear();
            }
        }
    });

    // 启动定时清理任务
    let store_cleanup = store.clone();
    let retention_days = config.retention_days;
    if retention_days > 0 {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            interval.tick().await;
            loop {
                interval.tick().await;
                let cutoff = chrono::Utc::now().timestamp_micros()
                    - (retention_days as i64 * 86400 * 1_000_000);
                match store_cleanup.cleanup(cutoff).await {
                    Ok(count) if count > 0 => {
                        #[cfg(feature = "log")]
                        ::tracing::info!("tracing cleanup: removed {} spans", count);
                    }
                    Err(e) => {
                        #[cfg(feature = "log")]
                        ::tracing::error!("tracing cleanup error: {}", e);
                    }
                    _ => {}
                }
            }
        });
    }

    Ok(TracingService {
        store,
        tx,
        broadcast,
        config: config.clone(),
    })
}

// ═══════════════════════════════════════════════════════════════
//  创建 Hook
// ═══════════════════════════════════════════════════════════════

/// 从 TracingService 创建 TracingHook
///
/// 在 `run()` 中调用，state move 之前 clone TracingService。
pub fn create_hook(svc: &TracingService) -> TracingHook {
    TracingHook {
        service_name: svc.config.service_name.clone(),
        tx: svc.tx.clone(),
    }
}

// ═══════════════════════════════════════════════════════════════
//  TracingHook：实现 afast::hook::Hook
// ═══════════════════════════════════════════════════════════════

/// 链路追踪 Hook
pub struct TracingHook {
    service_name: String,
    tx: mpsc::Sender<SpanData>,
}

impl afast::hook::Hook for TracingHook {
    fn before_request(
        &self,
        ctx: &afast::hook::RequestContext,
    ) -> Option<Box<dyn afast::hook::RequestGuard>> {
        // 检查 no_trace 属性，跳过不需要追踪的 handler
        if ctx.attrs.iter().any(|a| a.key == "no_trace") {
            return None;
        }

        let trace_id = generate_hex_id(32);
        let span_id = generate_hex_id(16);

        // 将 trace 上下文写入 per-request ctx，handler 可通过 Ctx<TraceContext> 提取
        ctx.ctx.insert(TraceContext {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            service_name: self.service_name.clone(),
            transport: ctx.transport.to_string(),
            handler_name: ctx.handler_name.to_string(),
            handler_desc: ctx.handler_desc.to_string(),
        });

        Some(Box::new(TracingRequestGuard {
            trace_id,
            span_id,
            handler_name: ctx.handler_name.to_string(),
            handler_desc: ctx.handler_desc.to_string(),
            transport: ctx.transport.to_string(),
            is_binary: ctx.is_binary,
            method: ctx.method.to_string(),
            long_connection: ctx.long_connection,
            service_name: self.service_name.clone(),
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
        }))
    }

    fn on_connect(
        &self,
        ctx: &afast::hook::RequestContext,
    ) -> Option<Box<dyn afast::hook::ConnectionGuard>> {
        // 检查 no_trace 属性（SSE/WS 等长连接也支持）
        if ctx.attrs.iter().any(|a| a.key == "no_trace") {
            return None;
        }

        let trace_id = generate_hex_id(32);
        let span_id = generate_hex_id(16);

        // 写入 trace 上下文，WS/SSE handler 可通过 Ctx<TraceContext> 提取
        ctx.ctx.insert(TraceContext {
            trace_id: trace_id.clone(),
            span_id: span_id.clone(),
            service_name: self.service_name.clone(),
            transport: ctx.transport.to_string(),
            handler_name: ctx.handler_name.to_string(),
            handler_desc: ctx.handler_desc.to_string(),
        });

        Some(Box::new(TracingConnectionGuard {
            trace_id,
            span_id,
            handler_name: ctx.handler_name.to_string(),
            handler_desc: ctx.handler_desc.to_string(),
            transport: ctx.transport.to_string(),
            is_binary: ctx.is_binary,
            method: ctx.method.to_string(),
            long_connection: ctx.long_connection,
            service_name: self.service_name.clone(),
            start: std::time::Instant::now(),
            tx: self.tx.clone(),
        }))
    }
}

// ═══════════════════════════════════════════════════════════════
//  Guards
// ═══════════════════════════════════════════════════════════════

struct TracingRequestGuard {
    trace_id: String,
    span_id: String,
    handler_name: String,
    handler_desc: String,
    transport: String,
    is_binary: bool,
    method: String,
    long_connection: bool,
    service_name: String,
    start: std::time::Instant,
    tx: mpsc::Sender<SpanData>,
}

impl afast::hook::RequestGuard for TracingRequestGuard {
    fn on_response(&mut self, _ctx: &afast::hook::RequestContext, _response: &[u8]) {
        let duration = self.start.elapsed();
        let span = SpanData {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: None,
            handler_name: self.handler_name.clone(),
            handler_desc: self.handler_desc.clone(),
            transport: self.transport.clone(),
            is_binary: self.is_binary,
            method: self.method.clone(),
            long_connection: self.long_connection,
            is_root: true,
            start_time: chrono::Utc::now().timestamp_micros() - duration.as_micros() as i64,
            duration_us: duration.as_micros() as i64,
            status: SpanStatus::Ok,
            error_code: None,
            error_message: None,
            service_name: self.service_name.clone(),
        };
        #[cfg(feature = "log")]
        ::tracing::info!(
            trace_id = %self.trace_id,
            span_id = %self.span_id,
            handler = %self.handler_name,
            transport = %self.transport,
            is_binary = self.is_binary,
            long_connection = self.long_connection,
            duration_us = duration.as_micros() as i64,
            "trace: span"
        );
        let _ = self.tx.try_send(span);
    }

    fn on_error(&mut self, _ctx: &afast::hook::RequestContext, error: &afast::Error) {
        let duration = self.start.elapsed();
        let span = SpanData {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: None,
            handler_name: self.handler_name.clone(),
            handler_desc: self.handler_desc.clone(),
            transport: self.transport.clone(),
            is_binary: self.is_binary,
            method: self.method.clone(),
            long_connection: self.long_connection,
            is_root: true,
            start_time: chrono::Utc::now().timestamp_micros() - duration.as_micros() as i64,
            duration_us: duration.as_micros() as i64,
            status: SpanStatus::Error,
            error_code: Some(error.code() as i32),
            error_message: Some(error.message().to_string()),
            service_name: self.service_name.clone(),
        };
        #[cfg(feature = "log")]
        ::tracing::warn!(
            trace_id = %self.trace_id,
            span_id = %self.span_id,
            handler = %self.handler_name,
            transport = %self.transport,
            is_binary = self.is_binary,
            long_connection = self.long_connection,
            duration_us = duration.as_micros() as i64,
            error_code = error.code() as i32,
            error_message = error.message(),
            "trace: span [ERROR]"
        );
        let _ = self.tx.try_send(span);
    }
}

struct TracingConnectionGuard {
    trace_id: String,
    span_id: String,
    handler_name: String,
    handler_desc: String,
    transport: String,
    is_binary: bool,
    method: String,
    long_connection: bool,
    service_name: String,
    start: std::time::Instant,
    tx: mpsc::Sender<SpanData>,
}

impl afast::hook::ConnectionGuard for TracingConnectionGuard {
    fn on_disconnect(&mut self, _ctx: &afast::hook::RequestContext) {
        let duration = self.start.elapsed();
        let span = SpanData {
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: None,
            handler_name: self.handler_name.clone(),
            handler_desc: self.handler_desc.clone(),
            transport: self.transport.clone(),
            is_binary: self.is_binary,
            method: self.method.clone(),
            long_connection: self.long_connection,
            is_root: true,
            start_time: chrono::Utc::now().timestamp_micros() - duration.as_micros() as i64,
            duration_us: duration.as_micros() as i64,
            status: SpanStatus::Ok,
            error_code: None,
            error_message: None,
            service_name: self.service_name.clone(),
        };
        #[cfg(feature = "log")]
        ::tracing::info!(
            trace_id = %self.trace_id,
            span_id = %self.span_id,
            handler = %self.handler_name,
            transport = %self.transport,
            is_binary = self.is_binary,
            long_connection = self.long_connection,
            duration_us = duration.as_micros() as i64,
            "trace: connection closed"
        );
        let _ = self.tx.try_send(span);
    }
}

// ═══════════════════════════════════════════════════════════════
//  辅助函数
// ═══════════════════════════════════════════════════════════════

/// 生成随机 hex ID
fn generate_hex_id(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..len / 2).map(|_| rng.r#gen()).collect();
    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 链路追踪配置扩展
pub trait AFasterTraceExt {
    /// 注册自定义 TraceStore
    ///
    /// 替换默认的 SQLite 存储。未注册时使用配置文件 `[tracing].db_path`。
    /// 两者都未配置时 `run()` 将 panic。
    fn set_trace_store(self, store: impl TraceStore + 'static) -> Self;
}

impl AFasterTraceExt for crate::AFaster {
    fn set_trace_store(mut self, store: impl TraceStore + 'static) -> Self {
        self.state.tracing.store = std::sync::Arc::new(store);
        self
    }
}
