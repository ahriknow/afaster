// ═══════════════════════════════════════════════════════════════
//  微信内容安全错误 (模块 07)
//  用户错误: 40701~40799
//  内部错误: 50701~50799
// ═══════════════════════════════════════════════════════════════

// ── 文本内容安全 ──────────────────────────────────────────────

/// 微信 API 返回业务错误 (40701)
#[inline]
pub fn api(errmsg: &str) -> crate::Error {
    crate::Error::custom(40701, errmsg)
}

/// 内容为空或超过 2500 字 (40702)
#[inline]
pub fn content_invalid() -> crate::Error {
    crate::Error::custom(40702, "Content is empty or exceeds 2500 characters")
}

/// 构建请求 URL 失败 (50701)
#[inline]
pub fn url() -> crate::Error {
    crate::Error::custom(50701, "Content check URL build failed")
}

/// 发送请求失败 (50702)
#[inline]
pub fn request() -> crate::Error {
    crate::Error::custom(50702, "Content check request failed")
}

/// 解析响应 JSON 失败 (50703)
#[inline]
pub fn response() -> crate::Error {
    crate::Error::custom(50703, "Content check response parse failed")
}

/// 反序列化响应失败 (50704)
#[inline]
pub fn parse() -> crate::Error {
    crate::Error::custom(50704, "Content check response deserialize failed")
}

/// 获取 access_token 失败 (50705)
#[inline]
pub fn token() -> crate::Error {
    crate::Error::custom(50705, "Failed to get access_token")
}

/// access_token 响应解析失败 (50706)
#[inline]
pub fn token_parse() -> crate::Error {
    crate::Error::custom(50706, "Access_token response parse failed")
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
