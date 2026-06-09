#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  JWT 令牌错误 (模块 01)
//  用户错误: 40101~40199
//  内部错误: 50101~50199
// ═══════════════════════════════════════════════════════════════

/// 令牌无效 (40101)
#[inline]
pub fn invalid_token() -> crate::Error {
    crate::Error::custom(40101, "Invalid token")
}

/// 令牌已过期 (40102)
#[inline]
pub fn token_expired() -> crate::Error {
    crate::Error::custom(40102, "Token expired")
}

/// JWT 编码失败 (50101)
#[inline]
pub fn encode_failed() -> crate::Error {
    crate::Error::custom(50101, "JWT encode failed")
}

/// JWT 验证失败 (50102)
#[inline]
pub fn verify_failed() -> crate::Error {
    crate::Error::custom(50102, "JWT verify failed")
}
