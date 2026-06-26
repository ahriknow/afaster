//! # 静态文件服务模块
//!
//! 提供静态网页内容服务，支持两种模式：
//! - **运行时目录模式**：从指定目录读取文件
//! - **编译期嵌入模式**：将整个目录在编译期嵌入二进制文件
//!
//! 通过 `get("*", serve)` 向前端提供静态内容（如 Vue SPA）。
//!
//! ## 配置示例 (config.toml)
//!
//! ```toml
//! [serve]
//! prefix = "/"           # URL 前缀, 默认 "/"
//! spa = true             # SPA 模式: 未找到文件时返回 index.html
//! ```
//!
//! ## 代码示例
//!
//! ```rust,no_run
//! // 运行时目录模式
//! let app = AFaster::new("config.toml".into()).await?
//!     .with_serve(afaster::serve::Serve::from_dir("./dist"))
//!     .service(svc)
//!     .run().await;
//!
//! // 编译期嵌入模式
//! let app = AFaster::new("config.toml".into()).await?
//!     .with_serve(afaster::serve::Serve::from_embedded(
//!         include_dir!("$CARGO_MANIFEST_DIR/dist")
//!     ))
//!     .service(svc)
//!     .run().await;
//! ```

use std::path::PathBuf;

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

/// Serve 模块配置（从 config.toml 反序列化）
#[derive(serde::Deserialize, Clone, Debug)]
pub struct ServeConfig {
    /// URL 前缀, 如 "/" 或 "/app", 默认 "/"
    pub prefix: Option<String>,
    /// SPA 模式: 未找到文件时返回 index.html, 默认 false
    #[serde(default)]
    pub spa: bool,
}

// ═══════════════════════════════════════════════════════════════
//  嵌入式目录类型
// ═══════════════════════════════════════════════════════════════

/// 编译期嵌入的目录数据类型
#[cfg(feature = "serve-embed")]
pub type EmbeddedDir = include_dir::Dir<'static>;

// ═══════════════════════════════════════════════════════════════
//  文件来源
// ═══════════════════════════════════════════════════════════════

/// 静态文件来源
#[derive(Clone)]
pub enum ServeSource {
    /// 从运行时目录读取
    Dir(PathBuf),
    /// 从编译期嵌入数据读取
    #[cfg(feature = "serve-embed")]
    Embedded(EmbeddedDir),
}

// ═══════════════════════════════════════════════════════════════
//  Serve 状态
// ═══════════════════════════════════════════════════════════════

/// 静态文件服务状态
///
/// 存储在 [`AppState`] 中，供 handler 访问。
#[derive(Clone)]
pub struct Serve {
    /// URL 前缀
    pub prefix: String,
    /// 是否启用 SPA 模式
    pub spa: bool,
    /// 文件来源
    pub source: ServeSource,
}

impl Serve {
    /// 从运行时目录创建
    ///
    /// # 示例
    /// ```rust,no_run
    /// let serve = Serve::from_dir("./dist");
    /// ```
    pub fn from_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            prefix: "/".to_string(),
            spa: false,
            source: ServeSource::Dir(dir.into()),
        }
    }

    /// 从编译期嵌入数据创建
    ///
    /// # 示例
    /// ```rust,no_run
    /// let serve = Serve::from_embedded(include_dir!("$CARGO_MANIFEST_DIR/dist"));
    /// ```
    #[cfg(feature = "serve-embed")]
    pub fn from_embedded(dir: EmbeddedDir) -> Self {
        Self {
            prefix: "/".to_string(),
            spa: false,
            source: ServeSource::Embedded(dir),
        }
    }

    /// 从配置创建（运行时目录模式）
    pub fn from_config(config: &ServeConfig, default_dir: &str) -> Self {
        Self {
            prefix: config.prefix.clone().unwrap_or_else(|| "/".to_string()),
            spa: config.spa,
            source: ServeSource::Dir(PathBuf::from(default_dir)),
        }
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let config: ServeConfig = crate::state::extract(table, "serve")?;
        Ok(Self::from_config(&config, "./static"))
    }

    /// 设置 URL 前缀
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// 启用/禁用 SPA 模式
    pub fn with_spa(mut self, spa: bool) -> Self {
        self.spa = spa;
        self
    }

    /// 获取泄露的路径字符串（用于 service! 宏注册路由）
    pub fn leaked_path(&self) -> &'static str {
        let path = if self.prefix == "/" || self.prefix.is_empty() {
            "*".to_string()
        } else {
            let trimmed = self.prefix.trim_start_matches('/');
            format!("{}/*", trimmed)
        };
        Box::leak(path.into_boxed_str())
    }

    /// 查找文件并返回 (内容, MIME 类型)
    pub fn resolve(&self, request_path: &str) -> Option<(Vec<u8>, String)> {
        let relative = strip_prefix(request_path, &self.prefix);

        // 尝试直接查找文件
        if let Some(result) = self.read_file(&relative) {
            return Some(result);
        }

        // SPA 模式: 尝试 index.html
        if self.spa
            && let Some(result) = self.read_file("index.html")
        {
            return Some(result);
        }

        None
    }

    /// 读取单个文件
    fn read_file(&self, relative: &str) -> Option<(Vec<u8>, String)> {
        // 规范化路径: 去掉开头的 /
        let relative = relative.trim_start_matches('/');

        // 如果路径为空或以 / 结尾, 尝试 index.html
        let path = if relative.is_empty() || relative.ends_with('/') {
            format!("{}index.html", relative)
        } else {
            relative.to_string()
        };

        // 安全检查: 防止路径遍历
        if path.contains("..") {
            return None;
        }

        let mime = mime_type(&path);

        match &self.source {
            ServeSource::Dir(dir) => {
                let full_path = dir.join(&path);
                match std::fs::read(&full_path) {
                    Ok(data) => Some((data, mime)),
                    Err(_) => None,
                }
            }
            #[cfg(feature = "serve-embed")]
            ServeSource::Embedded(dir) => match dir.get_file(&path) {
                Some(file) => Some((file.contents().to_vec(), mime)),
                None => None,
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  Handler
// ═══════════════════════════════════════════════════════════════

/// 静态文件服务 handler
///
/// 通过 `get("*", serve_handler)` 注册，处理所有 GET 请求。
#[afast::get(desc("Serve static files"), no_trace)]
pub async fn serve_handler(
    afast::FullPath(path): afast::FullPath,
    afast::State(state): afast::State<crate::AppState>,
) -> afast::HttpResult<afast::Serve> {
    let serve = state
        .serve
        .as_ref()
        .ok_or_else(|| afast::Error::custom(500, "Serve module not configured"))?;

    match serve.resolve(&path) {
        Some((data, content_type)) => Ok(afast::Serve { data, content_type }),
        None => Err(afast::Error::custom(404, "File not found")),
    }
}

// ═══════════════════════════════════════════════════════════════
//  辅助函数
// ═══════════════════════════════════════════════════════════════

/// 去掉路径前缀
fn strip_prefix(path: &str, prefix: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        return path.to_string();
    }

    let path = path.trim_start_matches('/');
    let prefix = prefix.trim_start_matches('/');

    if let Some(rest) = path.strip_prefix(prefix) {
        rest.to_string()
    } else {
        path.to_string()
    }
}

/// 根据文件扩展名推断 MIME 类型
fn mime_type(path: &str) -> String {
    // 优先使用 mime_guess
    let guess = mime_guess::from_path(path);
    if let Some(mime) = guess.first() {
        return mime.to_string();
    }

    // 回退到常见类型
    if path.ends_with(".wasm") {
        return "application/wasm".to_string();
    }
    if path.ends_with(".json") {
        return "application/json".to_string();
    }
    if path.ends_with(".svg") {
        return "image/svg+xml".to_string();
    }
    if path.ends_with(".webp") {
        return "image/webp".to_string();
    }
    if path.ends_with(".woff") {
        return "font/woff".to_string();
    }
    if path.ends_with(".woff2") {
        return "font/woff2".to_string();
    }

    "application/octet-stream".to_string()
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 静态文件服务配置扩展
pub trait AFasterServeExt {
    /// 设置静态文件服务
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// // 运行时目录模式
    /// AFaster::new("config.toml".into()).await?
    ///     .with_serve(afaster::serve::Serve::from_dir("./dist"))
    ///     .run().await;
    ///
    /// // 编译期嵌入模式
    /// AFaster::new("config.toml".into()).await?
    ///     .with_serve(afaster::serve::Serve::from_embedded(
    ///         include_dir!("$CARGO_MANIFEST_DIR/dist")
    ///     ))
    ///     .run().await;
    /// ```
    fn with_serve(self, serve: Serve) -> Self;
}

impl AFasterServeExt for crate::AFaster {
    fn with_serve(mut self, serve: Serve) -> Self {
        self.state.serve = Some(serve);
        self
    }
}
