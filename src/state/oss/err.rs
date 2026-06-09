#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  阿里云 OSS 错误 (模块 04)
//  用户错误: 40401~40499
//  内部错误: 50401~50499
// ═══════════════════════════════════════════════════════════════

/// HMAC-SHA256 初始化失败 (50401)
#[inline]
pub fn hmac_sha256() -> crate::Error {
    crate::Error::custom(50401, "HMAC-SHA256 init failed")
}

/// HMAC-SHA1 初始化失败 (50402)
#[inline]
pub fn hmac_sha1() -> crate::Error {
    crate::Error::custom(50402, "HMAC-SHA1 init failed")
}

/// 签名计算失败 (50403)
#[inline]
pub fn sign() -> crate::Error {
    crate::Error::custom(50403, "Signature calculation failed")
}

/// STS 请求发送失败 (50404)
#[inline]
pub fn sts_request() -> crate::Error {
    crate::Error::custom(50404, "STS request failed")
}

/// STS 响应错误 (50405)
#[inline]
pub fn sts_response() -> crate::Error {
    crate::Error::custom(50405, "STS response error")
}

/// STS 响应解析失败 (50406)
#[inline]
pub fn sts_parse() -> crate::Error {
    crate::Error::custom(50406, "STS response parse failed")
}

/// STS 未配置 role_arn (40401)
#[inline]
pub fn sts_no_role_arn() -> crate::Error {
    crate::Error::custom(40401, "STS role_arn not configured")
}
