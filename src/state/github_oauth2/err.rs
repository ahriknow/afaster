#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  GitHub OAuth2 错误 (模块 05)
//  用户错误: 40501~40599
//  内部错误: 50501~50599
// ═══════════════════════════════════════════════════════════════

// ── 内部错误 ────────────────────────────────────────────────

/// Token 请求发送失败 (50501)
#[inline]
pub fn exchange_request() -> crate::Error {
    crate::Error::custom(50501, "Token exchange request failed")
}

/// Token 响应解析失败 (50502)
#[inline]
pub fn exchange_parse() -> crate::Error {
    crate::Error::custom(50502, "Token exchange response parse failed")
}

/// 获取用户信息请求失败 (50503)
#[inline]
pub fn user_request() -> crate::Error {
    crate::Error::custom(50503, "User info request failed")
}

/// 用户信息解析失败 (50504)
#[inline]
pub fn user_parse() -> crate::Error {
    crate::Error::custom(50504, "User info response parse failed")
}

/// 获取用户邮箱请求失败 (50505)
#[inline]
pub fn email_request() -> crate::Error {
    crate::Error::custom(50505, "User email request failed")
}

/// 用户邮箱解析失败 (50506)
#[inline]
pub fn email_parse() -> crate::Error {
    crate::Error::custom(50506, "User email response parse failed")
}

/// 未注册回调函数 (50507)
#[inline]
pub fn no_callback() -> crate::Error {
    crate::Error::custom(50507, "GitHub callback not registered")
}

// ── 用户错误 ────────────────────────────────────────────────

/// Token 交换失败 / 认证失败 (40501)
#[inline]
pub fn exchange_auth() -> crate::Error {
    crate::Error::custom(40501, "GitHub authentication failed")
}

/// 未获取到 access_token (40502)
#[inline]
pub fn no_access_token() -> crate::Error {
    crate::Error::custom(40502, "GitHub authentication failed")
}

/// 回调缺少授权码 (40503)
#[inline]
pub fn no_code() -> crate::Error {
    crate::Error::custom(40503, "Missing authorization code")
}

/// 缺少 state 参数 (40504)
#[inline]
pub fn no_state() -> crate::Error {
    crate::Error::custom(40504, "Missing state parameter")
}

/// state 验证失败/已过期 (40505)
#[inline]
pub fn invalid_state() -> crate::Error {
    crate::Error::custom(40505, "Invalid or expired state")
}
