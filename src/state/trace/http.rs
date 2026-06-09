//! TraceStore 的 HTTP 实现
//!
//! 将 span 数据通过 HTTP 发送到远程服务。

use super::err;
use super::store::*;

/// HTTP 存储配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HttpTraceStoreConfig {
    /// 远程服务地址，如 "http://localhost:8080"
    pub url: String,
    /// API 路径前缀，默认 "/api/tracing"
    #[serde(default = "default_api_prefix")]
    pub api_prefix: String,
    /// 认证 Token（可选）
    pub token: Option<String>,
    /// 请求超时（秒），默认 5
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_api_prefix() -> String {
    "/api/tracing".to_string()
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

    fn url(&self, path: &str) -> String {
        format!("{}{}{}", self.config.url, self.config.api_prefix, path)
    }

    async fn post_json<T: serde::Serialize>(&self, path: &str, body: &T) -> crate::Result<()> {
        let mut req = self.client.post(self.url(path)).json(body);
        if let Some(ref token) = self.config.token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| err::http_request_failed(&format!("POST {}: {}", path, e)))?;

        if !resp.status().is_success() {
            return Err(err::http_request_failed(&format!(
                "POST {} returned {}",
                path,
                resp.status()
            )));
        }
        Ok(())
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> crate::Result<T> {
        let mut req = self.client.get(self.url(path));
        if let Some(ref token) = self.config.token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| err::http_request_failed(&format!("GET {}: {}", path, e)))?;

        if !resp.status().is_success() {
            return Err(err::http_request_failed(&format!(
                "GET {} returned {}",
                path,
                resp.status()
            )));
        }

        resp.json()
            .await
            .map_err(|e| err::http_request_failed(&format!("parse response: {}", e)))
    }
}

impl TraceStore for HttpTraceStore {
    fn insert_span(
        &self,
        span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move { self.post_json("/span", &span).await })
    }

    fn insert_spans(
        &self,
        spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move { self.post_json("/spans", &spans).await })
    }

    fn get_trace(
        &self,
        trace_id: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = crate::Result<Vec<SpanData>>> + Send + '_>,
    > {
        let path = format!("/trace/{}", trace_id);
        Box::pin(async move { self.get_json(&path).await })
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
