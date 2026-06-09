//! TraceStore trait：链路追踪存储抽象

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  数据类型
// ═══════════════════════════════════════════════════════════════

/// Span 状态
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    afast::AFastSerialize,
    afast::AFastDeserialize,
    afast::Tag,
)]
#[tag("Span 状态")]
pub enum SpanStatus {
    Ok,
    Error,
}

impl std::fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanStatus::Ok => write!(f, "ok"),
            SpanStatus::Error => write!(f, "error"),
        }
    }
}

/// 单个 Span 数据
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("Span 数据")]
pub struct SpanData {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub handler_name: String,
    pub handler_desc: String,
    pub transport: String,
    pub is_binary: bool,
    pub method: String,
    pub long_connection: bool,
    pub is_root: bool,
    pub start_time: i64,
    pub duration_us: i64,
    pub status: SpanStatus,
    pub error_code: Option<i32>,
    pub error_message: Option<String>,
    pub service_name: String,
}

/// Trace 摘要（列表展示用）
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("Trace 摘要")]
pub struct TraceSummary {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub root_handler: String,
    pub service_name: String,
    pub transport: String,
    pub is_binary: bool,
    pub method: String,
    pub long_connection: bool,
    pub start_time: i64,
    pub duration_us: i64,
    pub span_count: i64,
    pub status: SpanStatus,
    pub has_error: bool,
}

/// 列表查询过滤条件
#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    afast::AFastSerialize,
    afast::AFastDeserialize,
    afast::Tag,
)]
#[tag("Trace 过滤条件")]
pub struct TraceFilter {
    pub start_time_from: Option<i64>,
    pub start_time_to: Option<i64>,
    pub service_name: Option<String>,
    pub transport: Option<Vec<String>>,
    pub status: Option<SpanStatus>,
    pub min_duration_us: Option<i64>,
    pub max_duration_us: Option<i64>,
    pub error_code: Option<i32>,
    pub handler_name: Option<String>,
}

/// 分页列表结果
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("Trace 列表结果")]
pub struct TraceListResult {
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub data: Vec<TraceSummary>,
}

/// 子项分页结果
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("子项分页结果")]
pub struct ChildrenResult {
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub data: Vec<SpanData>,
}

/// 统计过滤条件
#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    afast::AFastSerialize,
    afast::AFastDeserialize,
    afast::Tag,
)]
#[tag("统计过滤条件")]
pub struct StatsFilter {
    pub start_time_from: Option<i64>,
    pub start_time_to: Option<i64>,
    pub service_name: Option<String>,
    pub transport: Option<String>,
}

/// 统计概览
#[derive(
    Debug, Clone, Serialize, Deserialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("统计概览")]
pub struct TraceStats {
    pub total_traces: i64,
    pub error_traces: i64,
    pub error_rate: f64,
    pub avg_duration_us: f64,
    pub p50_duration_us: i64,
    pub p95_duration_us: i64,
    pub p99_duration_us: i64,
    pub max_duration_us: i64,
    pub min_duration_us: i64,
}

// ═══════════════════════════════════════════════════════════════
//  TraceStore trait
// ═══════════════════════════════════════════════════════════════

/// 链路追踪存储 trait
///
/// 实现此 trait 可自定义存储后端（Redis、内存、ClickHouse 等）。
/// 默认提供 SQLite 实现。
///
/// 使用 `Pin<Box<dyn Future>>` 以支持动态分发（object-safe），
/// 用户可通过 `AFaster::set_trace_store(store)` 注册自定义存储。
pub trait TraceStore: Send + Sync + 'static {
    fn is_empty(&self) -> bool {
        false
    }
    fn insert_span(
        &self,
        span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>>;
    fn insert_spans(
        &self,
        spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>>;
    fn get_trace(
        &self,
        trace_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Vec<SpanData>>> + Send + '_>,
    >;
    fn list_traces(
        &self,
        filter: TraceFilter,
        page: i64,
        page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<TraceListResult>> + Send + '_>,
    >;
    fn stats(
        &self,
        filter: StatsFilter,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<TraceStats>> + Send + '_>>;
    fn cleanup(
        &self,
        before_ts: i64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<u64>> + Send + '_>>;

    /// 查询某个 span 的子项（分页）
    fn get_children(
        &self,
        trace_id: &str,
        parent_span_id: &str,
        page: i64,
        page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<ChildrenResult>> + Send + '_>,
    >;

    /// 查询单个 span
    fn get_span(
        &self,
        trace_id: &str,
        span_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Option<SpanData>>> + Send + '_>,
    >;
}

/// 为 `Box<dyn TraceStore>` 实现 `TraceStore`，支持动态分发
impl TraceStore for Box<dyn TraceStore> {
    fn insert_span(
        &self,
        span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        (**self).insert_span(span)
    }
    fn insert_spans(
        &self,
        spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        (**self).insert_spans(spans)
    }
    fn get_trace(
        &self,
        trace_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Vec<SpanData>>> + Send + '_>,
    > {
        (**self).get_trace(trace_id)
    }
    fn list_traces(
        &self,
        filter: TraceFilter,
        page: i64,
        page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<TraceListResult>> + Send + '_>,
    > {
        (**self).list_traces(filter, page, page_size)
    }
    fn stats(
        &self,
        filter: StatsFilter,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<TraceStats>> + Send + '_>>
    {
        (**self).stats(filter)
    }
    fn cleanup(
        &self,
        before_ts: i64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<u64>> + Send + '_>> {
        (**self).cleanup(before_ts)
    }
    fn get_children(
        &self,
        trace_id: &str,
        parent_span_id: &str,
        page: i64,
        page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<ChildrenResult>> + Send + '_>,
    > {
        (**self).get_children(trace_id, parent_span_id, page, page_size)
    }
    fn get_span(
        &self,
        trace_id: &str,
        span_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Option<SpanData>>> + Send + '_>,
    > {
        (**self).get_span(trace_id, span_id)
    }
}

/// 空的 Store
pub struct EmptyTraceStore;
impl TraceStore for EmptyTraceStore {
    fn is_empty(&self) -> bool {
        true
    }
    fn insert_span(
        &self,
        _span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
    fn insert_spans(
        &self,
        _spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
    fn get_trace(
        &self,
        _trace_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Vec<SpanData>>> + Send + '_>,
    > {
        Box::pin(async { Ok(vec![]) })
    }
    fn list_traces(
        &self,
        _filter: TraceFilter,
        _page: i64,
        _page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<TraceListResult>> + Send + '_>,
    > {
        Box::pin(async {
            Ok(TraceListResult {
                total: 0,
                page: 1,
                page_size: 20,
                data: vec![],
            })
        })
    }
    fn stats(
        &self,
        _filter: StatsFilter,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<TraceStats>> + Send + '_>>
    {
        Box::pin(async {
            Ok(TraceStats {
                total_traces: 0,
                error_traces: 0,
                error_rate: 0.0,
                avg_duration_us: 0.0,
                p50_duration_us: 0,
                p95_duration_us: 0,
                p99_duration_us: 0,
                max_duration_us: 0,
                min_duration_us: 0,
            })
        })
    }
    fn cleanup(
        &self,
        _before_ts: i64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<u64>> + Send + '_>> {
        Box::pin(async { Ok(0) })
    }
    fn get_children(
        &self,
        _trace_id: &str,
        _parent_span_id: &str,
        _page: i64,
        _page_size: i64,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<ChildrenResult>> + Send + '_>,
    > {
        Box::pin(async {
            Ok(ChildrenResult {
                total: 0,
                page: 1,
                page_size: 20,
                data: vec![],
            })
        })
    }
    fn get_span(
        &self,
        _trace_id: &str,
        _span_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Option<SpanData>>> + Send + '_>,
    > {
        Box::pin(async { Ok(None) })
    }
}
