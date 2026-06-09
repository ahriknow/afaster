#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  微信支付 V3 公共错误码 (模块 09)
//  用户错误: 40901~40999
//  内部错误: 50901~50999
// ═══════════════════════════════════════════════════════════════

#[inline]
pub fn private_key_load() -> crate::Error {
    crate::Error::custom(50901, "WxPay private key load failed")
}

#[inline]
pub fn sign_failed() -> crate::Error {
    crate::Error::custom(50902, "WxPay signature generation failed")
}

#[inline]
pub fn request_failed() -> crate::Error {
    crate::Error::custom(50903, "WxPay request failed")
}

#[inline]
pub fn prepay_request() -> crate::Error {
    crate::Error::custom(50904, "WxPay prepay request failed")
}

#[inline]
pub fn prepay_response() -> crate::Error {
    crate::Error::custom(50905, "WxPay prepay response parse failed")
}

#[inline]
pub fn api_error(code: &str, message: &str) -> crate::Error {
    crate::Error::custom(40902, format!("{}: {}", code, message))
}

#[inline]
pub fn query_response() -> crate::Error {
    crate::Error::custom(50907, "WxPay query order response parse failed")
}

#[inline]
pub fn refund_response() -> crate::Error {
    crate::Error::custom(50910, "WxPay refund response parse failed")
}

#[inline]
pub fn bill_response() -> crate::Error {
    crate::Error::custom(50914, "WxPay bill response parse failed")
}

#[inline]
pub fn config_missing(field: &str) -> crate::Error {
    crate::Error::custom(40905, format!("WxPay config missing: {}", field))
}

#[inline]
pub fn notify_decrypt(detail: &str) -> crate::Error {
    crate::Error::custom(
        40903,
        format!("WxPay notification decrypt failed: {}", detail),
    )
}

#[inline]
pub fn notify_verify() -> crate::Error {
    crate::Error::custom(40904, "WxPay notification signature verification failed")
}
