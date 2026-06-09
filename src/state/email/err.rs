#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  邮件模块错误 (模块 10)
//  用户错误: 41001~41099
//  内部错误: 51001~51099
// ═══════════════════════════════════════════════════════════════

/// 邮箱账户未找到 (41001)
#[inline]
pub fn account_not_found(id: &str) -> crate::Error {
    crate::Error::custom(41001, format!("Email account not found: {}", id))
}

/// 邮箱账户 ID 重复 (41002)
#[inline]
pub fn duplicate_account_id(id: &str) -> crate::Error {
    crate::Error::custom(41002, format!("Duplicate email account ID: {}", id))
}

/// 默认邮箱 ID 不存在 (41003)
#[inline]
pub fn default_not_found(id: &str) -> crate::Error {
    crate::Error::custom(41003, format!("Default email account ID not found: {}", id))
}

/// SMTP 连接失败 (51001)
#[inline]
pub fn smtp_connect_failed(msg: &str) -> crate::Error {
    crate::Error::custom(51001, format!("SMTP connection failed: {}", msg))
}

/// 邮件发送失败 (51002)
#[inline]
pub fn send_failed(msg: &str) -> crate::Error {
    crate::Error::custom(51002, format!("Email send failed: {}", msg))
}
