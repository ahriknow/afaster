#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  高德地图错误 (模块 13)
//  用户错误: 41301~41399
//  内部错误: 51301~51399
// ═══════════════════════════════════════════════════════════════

/// AMap key not configured (41301)
#[inline]
pub fn missing_key() -> crate::Error {
    crate::Error::custom(41301, "AMap key not configured")
}

/// AMap API request failed (51301)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51301, format!("AMap request failed: {}", detail))
}

/// AMap API response parse failed (51302)
#[inline]
pub fn response_parse_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51302, format!("AMap response parse failed: {}", detail))
}

/// AMap API business error (51303)
#[inline]
pub fn api_error(info: &str, infocode: &str) -> crate::Error {
    crate::Error::custom(51303, format!("AMap API error [{}]: {}", infocode, info))
}
