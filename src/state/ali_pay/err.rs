#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  支付宝错误码 (模块 26)
//  用户错误: 42601~42699
//  内部错误: 52601~52699
// ═══════════════════════════════════════════════════════════════

// ── 内部错误 ────────────────────────────────────────────────

/// 私钥加载失败 (52601)
#[inline]
pub fn private_key_load(detail: &str) -> crate::Error {
    crate::Error::custom(52601, format!("AliPay private key load failed: {}", detail))
}

/// 公钥加载失败 (52602)
#[inline]
pub fn public_key_load(detail: &str) -> crate::Error {
    crate::Error::custom(52602, format!("AliPay public key load failed: {}", detail))
}

/// 签名失败 (52603)
#[inline]
pub fn sign_failed(detail: &str) -> crate::Error {
    crate::Error::custom(52603, format!("AliPay sign failed: {}", detail))
}

/// 验签失败 (52604)
#[inline]
pub fn verify_failed(detail: &str) -> crate::Error {
    crate::Error::custom(52604, format!("AliPay verify failed: {}", detail))
}

/// 请求发送失败 (52605)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(52605, format!("AliPay request failed: {}", detail))
}

/// 响应解析失败 (52606)
#[inline]
pub fn response_parse_failed(detail: &str) -> crate::Error {
    crate::Error::custom(52606, format!("AliPay response parse failed: {}", detail))
}

/// 私钥未初始化 (52601)
#[inline]
pub fn private_key_not_init() -> crate::Error {
    crate::Error::custom(52601, "AliPay private key not initialized")
}

/// 公钥未配置 (52602)
#[inline]
pub fn public_key_not_configured() -> crate::Error {
    crate::Error::custom(52602, "AliPay public key not configured")
}

// ── 用户错误 ────────────────────────────────────────────────

/// 回调通知参数缺失 (42601)
#[inline]
pub fn notify_missing_params(detail: &str) -> crate::Error {
    crate::Error::custom(42601, format!("AliPay notify missing params: {}", detail))
}

/// 不支持的签名类型 (42602)
#[inline]
pub fn unsupported_sign_type(sign_type: &str) -> crate::Error {
    crate::Error::custom(
        42602,
        format!("AliPay unsupported sign type: {}", sign_type),
    )
}
