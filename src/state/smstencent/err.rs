#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  腾讯云短信错误 (模块 17)
//  用户错误: 41701~41799
//  内部错误: 51701~51799
// ═══════════════════════════════════════════════════════════════

/// 短信凭证未配置 (41701)
#[inline]
pub fn missing_credentials() -> crate::Error {
    crate::Error::custom(41701, "Tencent SMS credentials not configured")
}

/// HMAC-SHA256 初始化失败 (51701)
#[inline]
pub fn hmac_sha256() -> crate::Error {
    crate::Error::custom(51701, "HMAC-SHA256 init failed")
}

/// 短信 API 请求失败 (51702)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51702, format!("Tencent SMS request failed: {}", detail))
}

/// 短信回执回调未注册 (51703)
#[inline]
pub fn no_callback() -> crate::Error {
    crate::Error::custom(51703, "Tencent SMS report callback not registered")
}
