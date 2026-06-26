#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  微信支付错误 (模块 09)
//  用户错误: 40901~40999
//  内部错误: 50901~50999
// ═══════════════════════════════════════════════════════════════

// ── 用户错误 ────────────────────────────────────────────────

/// 配置缺少必要字段 (40901)
#[inline]
pub fn config_missing(field: &str) -> crate::Error {
    crate::Error::custom(40901, format!("WxPay config missing: {}", field))
}

/// 预下单 API 业务错误 (40902)
#[inline]
pub fn api_error(code: &str, message: &str) -> crate::Error {
    crate::Error::custom(40902, format!("{}: {}", code, message))
}

/// 回调通知验签失败 (40903)
#[inline]
pub fn notify_verify(detail: &str) -> crate::Error {
    crate::Error::custom(
        40903,
        format!(
            "WxPay notification signature verification failed: {}",
            detail
        ),
    )
}

// ── 内部错误 ────────────────────────────────────────────────

/// 私钥加载失败 (50901)
#[inline]
pub fn private_key_load(detail: &str) -> crate::Error {
    crate::Error::custom(50901, format!("WxPay private key load failed: {}", detail))
}

/// 签名生成失败 (50902)
#[inline]
pub fn sign_failed(detail: &str) -> crate::Error {
    crate::Error::custom(
        50902,
        format!("WxPay signature generation failed: {}", detail),
    )
}

/// 请求失败 (50903)
#[inline]
pub fn request_failed(detail: &str) -> crate::Error {
    crate::Error::custom(50903, format!("WxPay request failed: {}", detail))
}

/// 预下单请求失败 (50904)
#[inline]
pub fn prepay_request(detail: &str) -> crate::Error {
    crate::Error::custom(50904, format!("WxPay prepay request failed: {}", detail))
}

/// 预下单响应解析失败 (50905)
#[inline]
pub fn prepay_response(detail: &str) -> crate::Error {
    crate::Error::custom(
        50905,
        format!("WxPay prepay response parse failed: {}", detail),
    )
}

/// 查询订单响应解析失败 (50906)
#[inline]
pub fn query_response(detail: &str) -> crate::Error {
    crate::Error::custom(
        50906,
        format!("WxPay query order response parse failed: {}", detail),
    )
}

/// 退款响应解析失败 (50907)
#[inline]
pub fn refund_response(detail: &str) -> crate::Error {
    crate::Error::custom(
        50907,
        format!("WxPay refund response parse failed: {}", detail),
    )
}

/// 账单响应解析失败 (50908)
#[inline]
pub fn bill_response(detail: &str) -> crate::Error {
    crate::Error::custom(
        50908,
        format!("WxPay bill response parse failed: {}", detail),
    )
}
