//! TraceStore 的 TCP 实现
//!
//! 通过 TCP 连接将 span 数据发送到远程服务，使用 afastdata 序列化。

use super::err;
use super::store::*;
use afastdata::AFastSerialize;

/// TCP 存储配置
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TcpTraceStoreConfig {
    /// 远程服务地址，如 "127.0.0.1:9090"
    pub addr: String,
    /// 连接超时（秒），默认 5
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: u64,
    /// 重连间隔（秒），默认 3
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval: u64,
}

fn default_connect_timeout() -> u64 {
    5
}

fn default_reconnect_interval() -> u64 {
    3
}

/// TCP 存储实现
///
/// 通过 TCP 连接将 span 数据发送到远程服务。
/// 内部维护一个连接，断开时自动重连。
pub struct TcpTraceStore {
    config: TcpTraceStoreConfig,
    tx: tokio::sync::mpsc::UnboundedSender<Vec<SpanData>>,
}

impl TcpTraceStore {
    /// 创建 TCP 存储
    pub fn new(config: TcpTraceStoreConfig) -> crate::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let addr = config.addr.clone();
        let connect_timeout = config.connect_timeout;
        let reconnect_interval = config.reconnect_interval;

        // 后台 task：维护连接并发送数据
        tokio::spawn(async move {
            Self::run_connection(addr, connect_timeout, reconnect_interval, rx).await;
        });

        Ok(Self { config, tx })
    }

    async fn run_connection(
        addr: String,
        connect_timeout: u64,
        reconnect_interval: u64,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<SpanData>>,
    ) {
        loop {
            // 尝试连接
            let stream = match tokio::time::timeout(
                std::time::Duration::from_secs(connect_timeout),
                tokio::net::TcpStream::connect(&addr),
            )
            .await
            {
                Ok(Ok(stream)) => {
                    #[cfg(feature = "log")]
                    ::tracing::info!("trace tcp: connected to {}", addr);
                    stream
                }
                Ok(Err(e)) => {
                    #[cfg(feature = "log")]
                    ::tracing::warn!(
                        "trace tcp: connect to {} failed: {}, retrying in {}s",
                        addr,
                        e,
                        reconnect_interval
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(reconnect_interval)).await;
                    continue;
                }
                Err(_) => {
                    #[cfg(feature = "log")]
                    ::tracing::warn!(
                        "trace tcp: connect to {} timed out, retrying in {}s",
                        addr,
                        reconnect_interval
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(reconnect_interval)).await;
                    continue;
                }
            };

            let mut stream = stream;

            // 连接成功，开始接收并发送数据
            while let Some(spans) = rx.recv().await {
                // 序列化每个 span
                for span in &spans {
                    let data = span.to_bytes();

                    // 发送长度 + 数据
                    let len = (data.len() as u32).to_be_bytes();
                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut stream, &len).await {
                        #[cfg(feature = "log")]
                        ::tracing::warn!("trace tcp: write len failed: {}, reconnecting", e);
                        break;
                    }
                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut stream, &data).await {
                        #[cfg(feature = "log")]
                        ::tracing::warn!("trace tcp: write data failed: {}, reconnecting", e);
                        break;
                    }
                }
            }

            // channel 关闭，退出
            if rx.is_closed() {
                break;
            }

            #[cfg(feature = "log")]
            ::tracing::info!(
                "trace tcp: reconnecting to {} in {}s",
                addr,
                reconnect_interval
            );
            tokio::time::sleep(std::time::Duration::from_secs(reconnect_interval)).await;
        }
    }
}

impl TraceStore for TcpTraceStore {
    fn insert_span(
        &self,
        span: SpanData,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let _ = self.tx.send(vec![span]);
            Ok(())
        })
    }

    fn insert_spans(
        &self,
        spans: Vec<SpanData>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let _ = self.tx.send(spans);
            Ok(())
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
