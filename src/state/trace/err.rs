//! 链路追踪错误码
//!
//! 模块编号：28（528xx）

use crate::Error;

/// SQLite 初始化失败
pub fn sqlite_init_failed(detail: &str) -> Error {
    Error::custom(52801, format!("Tracing SQLite init failed: {}", detail))
}

/// SQLite 操作失败
pub fn sqlite_error(detail: &str) -> Error {
    Error::custom(52802, format!("Tracing SQLite error: {}", detail))
}

/// Span 数据格式错误
pub fn invalid_span_data(detail: &str) -> Error {
    Error::custom(42801, format!("Invalid span data: {}", detail))
}

/// Trace 不存在
pub fn trace_not_found(trace_id: &str) -> Error {
    Error::custom(42802, format!("Trace not found: {}", trace_id))
}

/// 未配置存储后端
pub fn no_trace_store() -> Error {
    Error::custom(
        52803,
        "Tracing store not configured: set [tracing].db_path, [tracing.http], or [tracing.tcp] in config, or call AFaster::set_trace_store()".to_string(),
    )
}

/// HTTP 存储初始化失败
pub fn http_init_failed(detail: &str) -> Error {
    Error::custom(52804, format!("Tracing HTTP init failed: {}", detail))
}

/// HTTP 请求失败
pub fn http_request_failed(detail: &str) -> Error {
    Error::custom(52805, format!("Tracing HTTP request failed: {}", detail))
}

/// TCP 存储初始化失败
pub fn tcp_init_failed(detail: &str) -> Error {
    Error::custom(52806, format!("Tracing TCP init failed: {}", detail))
}
