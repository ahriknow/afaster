#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  微信虚拟支付错误 (模块 06)
//  用户错误: 40601~40699
//  内部错误: 50601~50699
// ═══════════════════════════════════════════════════════════════

// ── 小程序登录 (获取 session_key 用于支付签名) ──────────────

/// 微信 API 登录业务错误 (40601)
#[inline]
pub fn login_api(errmsg: &str) -> crate::Error {
    crate::Error::custom(40601, errmsg)
}

/// session_key 已过期 (40602)
///
/// 微信 API 返回 errcode -41003 时使用
#[inline]
pub fn session_expired() -> crate::Error {
    crate::Error::custom(40602, "WeChat session_key expired, need re-login")
}

/// 构建登录请求 URL 失败 (50601)
#[inline]
pub fn login_url() -> crate::Error {
    crate::Error::custom(50601, "WxVirtualPay login URL build failed")
}

/// 发送登录请求失败 (50602)
#[inline]
pub fn login_request() -> crate::Error {
    crate::Error::custom(50602, "WxVirtualPay login request failed")
}

/// 解析登录响应失败 (50603)
#[inline]
pub fn login_response() -> crate::Error {
    crate::Error::custom(50603, "WxVirtualPay login response parse failed")
}

/// 反序列化登录响应失败 (50604)
#[inline]
pub fn login_parse() -> crate::Error {
    crate::Error::custom(50604, "WxVirtualPay login response deserialize failed")
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
