#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  数据库错误 (模块 25)
//  用户错误: 42501~42599
//  内部错误: 52501~52599
// ═══════════════════════════════════════════════════════════════

/// 数据库连接失败 (52501)
#[inline]
pub fn connect_failed(db_type: &str) -> crate::Error {
    crate::Error::custom(52501, format!("Database connection failed: {}", db_type))
}

/// 数据库配置解析失败 (52502)
#[inline]
pub fn config_error(msg: &str) -> crate::Error {
    crate::Error::custom(52502, format!("Database config error: {}", msg))
}
