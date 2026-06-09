#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  数据库错误 (模块 09)
//  用户错误: 40901~40999
//  内部错误: 50901~50999
// ═══════════════════════════════════════════════════════════════

/// 数据库连接失败 (50901)
#[inline]
pub fn connect_failed(db_type: &str) -> crate::Error {
    crate::Error::custom(50901, format!("Database connection failed: {}", db_type))
}

/// 数据库配置解析失败 (50902)
#[inline]
pub fn config_error(msg: &str) -> crate::Error {
    crate::Error::custom(50902, format!("Database config error: {}", msg))
}
