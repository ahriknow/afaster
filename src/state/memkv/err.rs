#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  MemKV 错误码 (模块 29)
//  用户错误: 42901~42999
//  内部错误: 52901~52999
// ═══════════════════════════════════════════════════════════════

/// 类型不匹配 (42901)
#[inline]
pub fn type_mismatch(expected: &str, actual: &str) -> crate::Error {
    crate::Error::custom(
        42901,
        format!("MemKV type mismatch: expected {}, got {}", expected, actual),
    )
}

/// Key 不存在 (42902)
#[inline]
pub fn key_not_found(key: &str) -> crate::Error {
    crate::Error::custom(42902, format!("MemKV key not found: {}", key))
}

/// 索引越界 (42903)
#[inline]
pub fn index_out_of_range(detail: &str) -> crate::Error {
    crate::Error::custom(42903, format!("MemKV index out of range: {}", detail))
}

/// 值不是有效数字 (42904)
#[inline]
pub fn not_a_number(detail: &str) -> crate::Error {
    crate::Error::custom(42904, format!("MemKV value is not a number: {}", detail))
}

/// 操作执行失败 (52901)
#[inline]
pub fn cmd_failed(cmd: &str, detail: &str) -> crate::Error {
    crate::Error::custom(52901, format!("MemKV {} failed: {}", cmd, detail))
}
