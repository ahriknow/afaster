#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  腾讯地图错误 (模块 14)
//  用户错误: 41401~41499
//  内部错误: 51401~51499
// ═══════════════════════════════════════════════════════════════

/// TMap key not configured (41401)
#[inline]
pub fn missing_key() -> crate::Error {
    crate::Error::custom(41401, "TMap key not configured")
}

/// TMap API request failed (51401)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51401, format!("TMap request failed: {}", detail))
}

/// TMap API response parse failed (51402)
#[inline]
pub fn response_parse_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51402, format!("TMap response parse failed: {}", detail))
}

/// TMap API business error (51403)
#[inline]
pub fn api_error(message: &str, status: i64) -> crate::Error {
    crate::Error::custom(51403, format!("TMap API error [{}]: {}", status, message))
}
