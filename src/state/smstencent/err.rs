#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  腾讯云短信错误 (模块 11)
//  用户错误: 41101~41199
//  内部错误: 51101~51199
// ═══════════════════════════════════════════════════════════════

/// SMS credentials not configured (41101)
#[inline]
pub fn missing_credentials() -> crate::Error {
    crate::Error::custom(41101, "Tencent SMS credentials not configured")
}

/// HMAC-SHA256 初始化失败 (51102)
#[inline]
pub fn hmac_sha256() -> crate::Error {
    crate::Error::custom(51102, "HMAC-SHA256 init failed")
}

/// SMS API request failed (51103)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51103, format!("Tencent SMS request failed: {}", detail))
}

/// SMS API response parse failed (51104)
#[inline]
pub fn response_parse_failed(detail: &str) -> crate::Error {
    crate::Error::custom(
        51104,
        format!("Tencent SMS response parse failed: {}", detail),
    )
}

/// SMS API error returned by provider (51105)
#[inline]
pub fn api_error(code: &str, message: &str) -> crate::Error {
    crate::Error::custom(
        51105,
        format!("Tencent SMS API error [{}]: {}", code, message),
    )
}

/// SMS report callback not registered (51107)
#[inline]
pub fn no_callback() -> crate::Error {
    crate::Error::custom(51107, "Tencent SMS report callback not registered")
}
