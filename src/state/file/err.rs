#![allow(dead_code)]

// ═══════════════════════════════════════════════════════════════
//  本地文件服务错误 (模块 19)
//  用户错误: 41901~41999
//  内部错误: 51901~51999
// ═══════════════════════════════════════════════════════════════

/// 文件不存在 (41901)
#[inline]
pub fn not_found(path: &str) -> crate::Error {
    crate::Error::custom(41901, format!("File not found: {}", path))
}

/// 文件过大 (41902)
#[inline]
pub fn too_large(size: u64, max: u64) -> crate::Error {
    crate::Error::custom(
        41902,
        format!("File too large: {} bytes (max {})", size, max),
    )
}

/// 文件类型不允许 (41903)
#[inline]
pub fn invalid_type(ext: &str) -> crate::Error {
    crate::Error::custom(41903, format!("File type not allowed: {}", ext))
}

/// 路径不合法 (41904)
#[inline]
pub fn invalid_path(path: &str) -> crate::Error {
    crate::Error::custom(41904, format!("Invalid file path: {}", path))
}

/// 路径遍历攻击 (41905)
#[inline]
pub fn path_traversal(path: &str) -> crate::Error {
    crate::Error::custom(41905, format!("Path traversal detected: {}", path))
}

/// 文件读取失败 (51901)
#[inline]
pub fn read_failed(path: &str, detail: &str) -> crate::Error {
    crate::Error::custom(51901, format!("Failed to read {}: {}", path, detail))
}

/// 文件写入失败 (51902)
#[inline]
pub fn write_failed(path: &str, detail: &str) -> crate::Error {
    crate::Error::custom(51902, format!("Failed to write {}: {}", path, detail))
}

/// 文件删除失败 (51903)
#[inline]
pub fn delete_failed(path: &str, detail: &str) -> crate::Error {
    crate::Error::custom(51903, format!("Failed to delete {}: {}", path, detail))
}

/// 目录创建失败 (51904)
#[inline]
pub fn mkdir_failed(path: &str, detail: &str) -> crate::Error {
    crate::Error::custom(51904, format!("Failed to create dir {}: {}", path, detail))
}

/// 目录读取失败 (51905)
#[inline]
pub fn list_failed(path: &str, detail: &str) -> crate::Error {
    crate::Error::custom(51905, format!("Failed to list dir {}: {}", path, detail))
}
