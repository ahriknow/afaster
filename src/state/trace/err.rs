//! 链路追踪错误码
//!
//! 模块编号：30（430xx / 530xx）

use crate::Error;

/// Span 数据格式错误 (43001)
pub fn invalid_span_data(detail: &str) -> Error {
    Error::custom(43001, format!("Invalid span data: {}", detail))
}

/// Trace 不存在 (43002)
pub fn trace_not_found(trace_id: &str) -> Error {
    Error::custom(43002, format!("Trace not found: {}", trace_id))
}

/// SQLite 初始化失败 (53001)
pub fn sqlite_init_failed(detail: &str) -> Error {
    Error::custom(53001, format!("Tracing SQLite init failed: {}", detail))
}

/// SQLite 操作失败 (53002)
pub fn sqlite_error(detail: &str) -> Error {
    Error::custom(53002, format!("Tracing SQLite error: {}", detail))
}

/// 未配置存储后端 (53003)
pub fn no_trace_store() -> Error {
    Error::custom(
        53003,
        "Tracing store not configured: set [tracing].db_path, [tracing.http], or [tracing.tcp] in config, or call AFaster::set_trace_store()".to_string(),
    )
}

/// HTTP 存储初始化失败 (53004)
pub fn http_init_failed(detail: &str) -> Error {
    Error::custom(53004, format!("Tracing HTTP init failed: {}", detail))
}

/// HTTP 请求失败 (53005)
pub fn http_request_failed(detail: &str) -> Error {
    Error::custom(53005, format!("Tracing HTTP request failed: {}", detail))
}

/// TCP 存储初始化失败 (53006)
pub fn tcp_init_failed(detail: &str) -> Error {
    Error::custom(53006, format!("Tracing TCP init failed: {}", detail))
}
