#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  阿里云短信错误 (模块 11)
//  用户错误: 41101~41199
//  内部错误: 51101~51199
// ═══════════════════════════════════════════════════════════════

/// 短信凭证未配置 (41101)
#[inline]
pub fn missing_credentials() -> crate::Error {
    crate::Error::custom(41101, "Alibaba SMS credentials not configured")
}

/// HMAC-SHA1 初始化失败 (51101)
#[inline]
pub fn hmac_sha1() -> crate::Error {
    crate::Error::custom(51101, "HMAC-SHA1 init failed")
}

/// 短信 API 请求失败 (51102)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51102, format!("Alibaba SMS request failed: {}", detail))
}

/// 短信回执回调未注册 (51103)
#[inline]
pub fn no_callback() -> crate::Error {
    crate::Error::custom(51103, "Alibaba SMS report callback not registered")
}
