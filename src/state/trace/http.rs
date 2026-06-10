//! TraceStore 的 HTTP 实现
//!
//! 将 span 数据通过 HTTP 发送到远程服务，使用 afastdata 二进制协议。

use super::err;
use super::handler::ReportBatchReq;
use super::store::*;
use afastdata::AFastSerialize;

/// HTTP 存储配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HttpTraceStoreConfig {
    /// 远程服务地址，如 "http://localhost:8080"
    pub url: String,
    /// API 路径，默认 "/tracing/report"
    #[serde(default = "default_path")]
    pub path: String,
    /// 认证 Token（可选）
    pub token: Option<String>,
    /// 请求超时（秒），默认 5
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_path() -> String {
    "/tracing/report".to_string()
}

fn default_timeout() -> u64 {
    5
}

/// HTTP 存储实现
#[derive(Clone)]
pub struct HttpTraceStore {
    client: reqwest::Client,
    config: HttpTraceStoreConfig,
}

impl HttpTraceStore {
    /// 创建 HTTP 存储
    pub fn new(config: HttpTraceStoreConfig) -> crate::Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(|e| err::http_init_failed(&format!("build client: {}", e)))?;

        Ok(Self { client, config })
    }

    fn url(&self) -> String {
        format!(
            "{}{}",
            self.config.url.trim_end_matches('/'),
            self.config.path
        )
    }

    async fn post_binary(&self, data: Vec<u8>) -> crate::Result<()> {
        let mut req = self
            .client
            .post(self.url())
            .header("Content-Type", "application/octet-stream")
            .body(data);
        if let Some(ref token) = self.config.token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| err::http_request_failed(&format!("POST: {}", e)))?;

        if !resp.status().is_success() {
            return Err(err::http_request_failed(&format!(
                "POST returned {}",
                resp.status()
            )));
        }
        Ok(())
    }
}

impl TraceStore for HttpTraceStore {
    fn insert_span(
        &self,
        span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let req = ReportBatchReq { spans: vec![span] };
            self.post_binary(req.to_bytes()).await
        })
    }

    fn insert_spans(
        &self,
        spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let req = ReportBatchReq { spans };
            self.post_binary(req.to_bytes()).await
        })
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
        // HTTP store 不支持本地查询，返回空结果
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
