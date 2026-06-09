//! Binary handlers：上报、查询、统计

use afast::handler;
use afast::{Data, State};
use serde_json;

use super::store::{SpanData, StatsFilter, TraceFilter};
use crate::AppState;

// ═══════════════════════════════════════════════════════════════
//  请求/响应类型
// ═══════════════════════════════════════════════════════════════

#[derive(
    serde::Deserialize, serde::Serialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("上报请求")]
pub struct ReportBatchReq {
    pub spans: Vec<SpanData>,
}

#[derive(serde::Serialize, afast::AFastSerialize, afast::Tag)]
#[tag("上报响应")]
pub struct ReportResp {
    pub code: i32,
    pub message: String,
}

#[derive(
    serde::Deserialize, serde::Serialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("查询子项请求")]
pub struct GetChildrenReq {
    pub trace_id: String,
    pub parent_span_id: String,
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

#[derive(serde::Serialize, afast::AFastSerialize, afast::Tag)]
#[tag("查询子项响应")]
pub struct GetChildrenResp {
    pub code: i32,
    pub data: super::store::ChildrenResult,
}

#[derive(
    serde::Deserialize, serde::Serialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("查询 Trace 请求")]
pub struct GetTraceReq {
    pub trace_id: String,
}

#[derive(serde::Serialize, afast::AFastSerialize, afast::Tag)]
#[tag("查询 Trace 响应")]
pub struct GetTraceResp {
    pub trace_id: String,
    pub spans: Vec<SpanData>,
}

#[derive(
    serde::Deserialize, serde::Serialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("Trace 列表请求")]
pub struct ListTracesReq {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
    pub filter: Option<TraceFilter>,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}

#[derive(serde::Serialize, afast::AFastSerialize, afast::Tag)]
#[tag("Trace 列表响应")]
pub struct ListTracesResp {
    pub code: i32,
    pub data: super::store::TraceListResult,
}

#[derive(
    serde::Deserialize, serde::Serialize, afast::AFastSerialize, afast::AFastDeserialize, afast::Tag,
)]
#[tag("统计请求")]
pub struct StatsReq {
    pub filter: Option<StatsFilter>,
}

#[derive(serde::Serialize, afast::AFastSerialize, afast::Tag)]
#[tag("统计响应")]
pub struct StatsResp {
    pub code: i32,
    pub data: super::store::TraceStats,
}

// ═══════════════════════════════════════════════════════════════
//  Handlers
// ═══════════════════════════════════════════════════════════════

#[handler(desc("上报 span"), no_trace)]
pub async fn report(
    State(state): State<AppState>,
    Data(span): Data<SpanData>,
) -> afast::Result<ReportResp> {
    state
        .tracing
        .store()
        .insert_span(span)
        .await
        .map_err(|e| crate::Error::custom(52802, format!("insert span failed: {}", e)))?;
    Ok(ReportResp {
        code: 0,
        message: "ok".into(),
    })
}

#[handler(desc("批量上报 spans"), no_trace)]
pub async fn report_batch(
    State(state): State<AppState>,
    Data(req): Data<ReportBatchReq>,
) -> afast::Result<ReportResp> {
    state
        .tracing
        .store()
        .insert_spans(req.spans)
        .await
        .map_err(|e| crate::Error::custom(52802, format!("insert spans failed: {}", e)))?;
    Ok(ReportResp {
        code: 0,
        message: "ok".into(),
    })
}

#[handler(desc("查询 trace 详情"), no_trace)]
pub async fn get_trace(
    State(state): State<AppState>,
    Data(req): Data<GetTraceReq>,
) -> afast::Result<GetTraceResp> {
    let spans = state
        .tracing
        .store()
        .get_trace(&req.trace_id)
        .await
        .map_err(|e| crate::Error::custom(52802, format!("get trace failed: {}", e)))?;
    if spans.is_empty() {
        return Err(
            crate::Error::custom(42802, format!("Trace not found: {}", req.trace_id)).into(),
        );
    }
    Ok(GetTraceResp {
        trace_id: req.trace_id,
        spans,
    })
}

#[handler(desc("查询 trace 列表"), no_trace)]
pub async fn list_traces(
    State(state): State<AppState>,
    Data(req): Data<ListTracesReq>,
) -> afast::Result<ListTracesResp> {
    let page = req.page.max(1);
    let page_size = req.page_size.clamp(1, 100);
    let filter = req.filter.unwrap_or_default();
    let result = state
        .tracing
        .store()
        .list_traces(filter, page, page_size)
        .await
        .map_err(|e| crate::Error::custom(52802, format!("list traces failed: {}", e)))?;
    Ok(ListTracesResp {
        code: 0,
        data: result,
    })
}

#[handler(desc("查询统计"), no_trace)]
pub async fn stats(
    State(state): State<AppState>,
    Data(req): Data<StatsReq>,
) -> afast::Result<StatsResp> {
    let filter = req.filter.unwrap_or_default();
    let result = state
        .tracing
        .store()
        .stats(filter)
        .await
        .map_err(|e| crate::Error::custom(52802, format!("stats failed: {}", e)))?;
    Ok(StatsResp {
        code: 0,
        data: result,
    })
}

// ═══════════════════════════════════════════════════════════════
//  子项查询
// ═══════════════════════════════════════════════════════════════

#[handler(desc("查询子 span"), no_trace)]
pub async fn get_children(
    State(state): State<AppState>,
    Data(req): Data<GetChildrenReq>,
) -> afast::Result<GetChildrenResp> {
    let page = req.page.max(1);
    let page_size = req.page_size.clamp(1, 100);
    let result = state
        .tracing
        .store()
        .get_children(&req.trace_id, &req.parent_span_id, page, page_size)
        .await
        .map_err(|e| crate::Error::custom(52802, format!("get_children failed: {}", e)))?;
    Ok(GetChildrenResp {
        code: 0,
        data: result,
    })
}

// ═══════════════════════════════════════════════════════════════
//  SSE 实时推送
// ═══════════════════════════════════════════════════════════════

/// 实时 trace 事件推送（SSE）
///
/// 客户端通过 EventSource 连接此端点，新 trace 到达时自动推送。
#[afast::sse(desc("实时 trace 事件"), no_trace)]
pub async fn trace_events(
    State(state): State<AppState>,
    sender: afast::SseSender,
) -> afast::Result<()> {
    let mut rx = state.tracing.subscribe();
    // 发送初始心跳
    let _ = sender
        .send_event("connected", &serde_json::json!({"status": "ok"}))
        .await;

    loop {
        match rx.recv().await {
            Ok(event) => {
                if sender.send_event("trace", &event).await.is_err() {
                    break; // 客户端断开
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                continue; // 丢弃落后消息
            }
            Err(_) => break,
        }
    }
    Ok(())
}
