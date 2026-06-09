#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  Redis / Valkey 错误 (模块 16)
//  用户错误: 41601~41699
//  内部错误: 51601~51699
// ═══════════════════════════════════════════════════════════════

/// Redis 连接失败 (51601)
#[inline]
pub fn connect_failed(detail: &str) -> crate::Error {
    crate::Error::custom(51601, format!("Redis connect failed: {}", detail))
}

/// Redis 连接断开 (51602)
#[inline]
pub fn connection_lost(detail: &str) -> crate::Error {
    crate::Error::custom(51602, format!("Redis connection lost: {}", detail))
}

/// Redis 命令执行失败 (51603)
#[inline]
pub fn cmd_failed(cmd: &str, detail: &str) -> crate::Error {
    crate::Error::custom(51603, format!("Redis {} failed: {}", cmd, detail))
}
