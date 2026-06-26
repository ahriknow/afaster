#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  腾讯云 COS 错误 (模块 12)
//  用户错误: 41201~41299
//  内部错误: 51201~51299
// ═══════════════════════════════════════════════════════════════

/// HMAC-SHA256 初始化失败 (51201)
#[inline]
pub fn hmac_sha256() -> crate::Error {
    crate::Error::custom(51201, "HMAC-SHA256 init failed")
}

/// 签名计算失败 (51202)
#[inline]
pub fn sign_failed() -> crate::Error {
    crate::Error::custom(51202, "Signature calculation failed")
}

/// 请求发送失败 (51203)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51203, format!("COS request failed: {}", detail))
}

/// 响应解析失败 (51204)
#[inline]
pub fn response_parse_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51204, format!("COS response parse failed: {}", detail))
}
