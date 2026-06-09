//! 本地文件服务模块
//!
//! 提供文件上传、下载、删除、目录列表等功能。
//! 支持文件类型校验、大小限制、路径安全检查。
//!
//! # 配置
//!
//! ```toml
//! [file]
//! root = "./uploads"                    # 存储根目录
//! max_size = 10485760                   # 最大文件大小（字节），默认 10MB
//! allowed = ["jpg","png","gif","pdf"]   # 允许的扩展名，为空则不限制
//! url_prefix = "/files"                 # 访问 URL 前缀
//! ```

mod err;
use err::*;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ═══════════════════════════════════════════════════════════════
//  公开类型
// ═══════════════════════════════════════════════════════════════

/// 文件元信息
#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    /// 相对路径（相对于 root）
    pub path: String,
    /// 文件名
    pub name: String,
    /// 扩展名（小写）
    pub ext: String,
    /// 文件大小（字节）
    pub size: u64,
    /// MIME 类型
    pub mime: String,
    /// 是否为目录
    pub is_dir: bool,
}

/// 目录列表
#[derive(Debug, Clone, Serialize)]
pub struct DirList {
    /// 当前目录路径
    pub path: String,
    /// 子项列表
    pub entries: Vec<FileInfo>,
}

fn default_root() -> String {
    "./uploads".to_string()
}
fn default_max_size() -> u64 {
    10 * 1024 * 1024 // 10MB
}
fn default_url_prefix() -> String {
    "/files".to_string()
}

/// 文件服务配置
#[derive(Clone, Deserialize)]
pub struct FileConfig {
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default = "default_max_size")]
    pub max_size: u64,
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default = "default_url_prefix")]
    pub url_prefix: String,
}

/// 本地文件服务
#[derive(Clone)]
pub struct FileService {
    root: PathBuf,
    max_size: u64,
    allowed: Vec<String>,
    url_prefix: String,
}

// ═══════════════════════════════════════════════════════════════
//  实现
// ═══════════════════════════════════════════════════════════════

impl FileService {
    /// 从配置创建文件服务，自动创建根目录
    pub fn new(config: &FileConfig) -> crate::Result<Self> {
        let root = PathBuf::from(&config.root);

        // 创建根目录（如果不存在）
        if !root.exists() {
            std::fs::create_dir_all(&root)
                .map_err(|e| mkdir_failed(&config.root, &e.to_string()))?;
        }

        let root = root.canonicalize().map_err(|e| {
            crate::Error::custom(51904, format!("Failed to resolve root path: {}", e))
        })?;

        Ok(Self {
            root,
            max_size: config.max_size,
            allowed: config
                .allowed
                .iter()
                .map(|s| s.to_lowercase().trim_start_matches('.').to_string())
                .collect(),
            url_prefix: config.url_prefix.trim_end_matches('/').to_string(),
        })
    }

    // ── 上传 ──────────────────────────────────────────────────────

    /// 上传文件
    ///
    /// - `path`: 相对路径，如 `"avatars/1.jpg"`
    /// - `data`: 文件内容
    pub fn upload(&self, path: &str, data: &[u8]) -> crate::Result<FileInfo> {
        // 安全检查
        let safe_path = self.validate_path(path)?;

        // 大小检查
        if data.len() as u64 > self.max_size {
            return Err(too_large(data.len() as u64, self.max_size));
        }

        // 类型检查
        let ext = get_ext(path);
        if !self.allowed.is_empty() && !self.allowed.iter().any(|a| a == &ext) {
            return Err(invalid_type(&ext));
        }

        // 创建父目录
        if let Some(parent) = safe_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| mkdir_failed(path, &e.to_string()))?;
            }
        }

        // 写入文件
        std::fs::write(&safe_path, data).map_err(|e| write_failed(path, &e.to_string()))?;

        Ok(self.build_info(&safe_path)?)
    }

    // ── 下载 ──────────────────────────────────────────────────────

    /// 下载文件（返回字节）
    pub fn download(&self, path: &str) -> crate::Result<Vec<u8>> {
        let safe_path = self.resolve_path(path)?;

        if !safe_path.exists() {
            return Err(not_found(path));
        }
        if safe_path.is_dir() {
            return Err(invalid_path(path));
        }

        std::fs::read(&safe_path).map_err(|e| read_failed(path, &e.to_string()))
    }

    // ── 删除 ──────────────────────────────────────────────────────

    /// 删除文件或目录
    ///
    /// - 文件：直接删除
    /// - 目录：递归删除
    pub fn delete(&self, path: &str) -> crate::Result<()> {
        let safe_path = self.resolve_path(path)?;

        if !safe_path.exists() {
            return Err(not_found(path));
        }

        if safe_path.is_dir() {
            std::fs::remove_dir_all(&safe_path).map_err(|e| delete_failed(path, &e.to_string()))?;
        } else {
            std::fs::remove_file(&safe_path).map_err(|e| delete_failed(path, &e.to_string()))?;
        }

        Ok(())
    }

    // ── 文件信息 ──────────────────────────────────────────────────

    /// 获取文件/目录信息
    pub fn info(&self, path: &str) -> crate::Result<FileInfo> {
        let safe_path = self.resolve_path(path)?;

        if !safe_path.exists() {
            return Err(not_found(path));
        }

        self.build_info(&safe_path)
    }

    /// 文件/目录是否存在
    pub fn exists(&self, path: &str) -> bool {
        if let Ok(safe_path) = self.resolve_path(path) {
            safe_path.exists()
        } else {
            false
        }
    }

    // ── 目录列表 ──────────────────────────────────────────────────

    /// 列出目录内容
    pub fn list(&self, dir: &str) -> crate::Result<DirList> {
        let safe_path = if dir.is_empty() || dir == "/" {
            self.root.clone()
        } else {
            self.resolve_path(dir)?
        };

        if !safe_path.exists() {
            return Err(not_found(dir));
        }
        if !safe_path.is_dir() {
            return Err(invalid_path(dir));
        }

        let mut entries = Vec::new();
        let read_dir =
            std::fs::read_dir(&safe_path).map_err(|e| list_failed(dir, &e.to_string()))?;

        for entry in read_dir {
            let entry = entry.map_err(|e| list_failed(dir, &e.to_string()))?;
            let info = self.build_info(&entry.path())?;
            entries.push(info);
        }

        // 按名称排序
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let rel_path = self.relative_path(&safe_path);

        Ok(DirList {
            path: rel_path,
            entries,
        })
    }

    // ── URL ───────────────────────────────────────────────────────

    /// 生成文件访问 URL
    pub fn url(&self, path: &str) -> crate::Result<String> {
        let safe_path = self.resolve_path(path)?;

        if !safe_path.exists() {
            return Err(not_found(path));
        }

        let rel = self.relative_path(&safe_path);
        Ok(format!("{}/{}", self.url_prefix, rel))
    }

    // ── 内部辅助 ──────────────────────────────────────────────────

    /// 校验并解析路径（防路径遍历）
    fn validate_path(&self, path: &str) -> crate::Result<PathBuf> {
        let path = path.trim().trim_start_matches('/');

        if path.is_empty() {
            return Err(invalid_path(path));
        }

        // 检查路径遍历
        if path.contains("..") || path.contains('\0') {
            return Err(path_traversal(path));
        }

        let full = self.root.join(path);

        // canonicalize 后检查是否在 root 下
        // 注意：文件可能还不存在，所以检查父目录
        if let Some(parent) = full.parent() {
            if parent.exists() {
                let canon_parent = parent
                    .canonicalize()
                    .map_err(|e| invalid_path(&format!("{}: {}", path, e)))?;
                if !canon_parent.starts_with(&self.root) {
                    return Err(path_traversal(path));
                }
            }
        }

        Ok(full)
    }

    /// 解析已存在文件的路径
    fn resolve_path(&self, path: &str) -> crate::Result<PathBuf> {
        let path = path.trim().trim_start_matches('/');

        if path.is_empty() {
            return Ok(self.root.clone());
        }

        if path.contains("..") || path.contains('\0') {
            return Err(path_traversal(path));
        }

        let full = self.root.join(path);

        // 文件必须存在，且 canonicalize 后必须在 root 下
        if full.exists() {
            let canon = full
                .canonicalize()
                .map_err(|e| invalid_path(&format!("{}: {}", path, e)))?;
            if !canon.starts_with(&self.root) {
                return Err(path_traversal(path));
            }
            Ok(canon)
        } else {
            // 不存在时做前缀检查
            Ok(full)
        }
    }

    /// 构建 FileInfo
    fn build_info(&self, full_path: &Path) -> crate::Result<FileInfo> {
        let name = full_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = get_ext(&name);
        let rel = self.relative_path(full_path);
        let is_dir = full_path.is_dir();

        let size = if is_dir {
            0
        } else {
            std::fs::metadata(full_path).map(|m| m.len()).unwrap_or(0)
        };

        let mime = mime_from_ext(&ext);

        Ok(FileInfo {
            path: rel,
            name,
            ext,
            size,
            mime: mime.to_string(),
            is_dir,
        })
    }

    /// 计算相对路径
    fn relative_path(&self, full: &Path) -> String {
        full.strip_prefix(&self.root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| full.to_string_lossy().to_string())
    }
}

// ═══════════════════════════════════════════════════════════════
//  工具函数
// ═══════════════════════════════════════════════════════════════

/// 获取小写扩展名（不含点）
fn get_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// 根据扩展名返回 MIME 类型
fn mime_from_ext(ext: &str) -> &'static str {
    match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "7z" => "application/x-7z-compressed",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "md" => "text/markdown",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "webm" => "video/webm",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}
