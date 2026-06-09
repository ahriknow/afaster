#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  微信虚拟支付错误 (模块 06)
//  用户错误: 40601~40699
//  内部错误: 50601~50699
// ═══════════════════════════════════════════════════════════════

// ── 独立登录 ────────────────────────────────────────────────

/// 构建请求 URL 失败 (50601)
#[inline]
pub fn login_url() -> crate::Error {
    crate::Error::custom(50601, "Virtual pay login URL build failed")
}

/// 发送登录请求失败 (50602)
#[inline]
pub fn login_request() -> crate::Error {
    crate::Error::custom(50602, "Virtual pay login request failed")
}

/// 解析登录响应 JSON 失败 (50603)
#[inline]
pub fn login_response() -> crate::Error {
    crate::Error::custom(50603, "Virtual pay login response parse failed")
}

/// 微信 API 返回业务错误 (40601)
#[inline]
pub fn login_api(errmsg: &str) -> crate::Error {
    crate::Error::custom(40601, errmsg)
}

/// 反序列化登录响应失败 (50604)
#[inline]
pub fn login_parse() -> crate::Error {
    crate::Error::custom(50604, "Virtual pay login response deserialize failed")
}

// ── 通用 ────────────────────────────────────────────────────

/// session_key 已过期 (40602)
///
/// 微信 API 返回 errcode -41003 时使用
#[inline]
pub fn session_expired() -> crate::Error {
    crate::Error::custom(40602, "WeChat login session expired")
}

// ── 回调通知 ────────────────────────────────────────────────

/// 缺少验签参数 (40811)
#[inline]
pub fn missing_verify_param() -> crate::Error {
    crate::Error::custom(40811, "Missing signature verification parameter")
}

/// 签名验证失败 (40812)
#[inline]
pub fn signature_mismatch() -> crate::Error {
    crate::Error::custom(40812, "Signature verification failed")
}

/// 消息体解密失败 (40813)
#[inline]
pub fn decrypt_failed(detail: &str) -> crate::Error {
    crate::Error::custom(40813, format!("Message body decryption failed: {}", detail))
}
