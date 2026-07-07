//! # 静态文件服务模块
//!
//! 提供静态网页内容服务，支持两种模式：
//! - **运行时目录模式**：从指定目录读取文件
//! - **编译期嵌入模式**：将整个目录在编译期嵌入二进制文件
//!
//! 通过 `get("*", serve)` 向前端提供静态内容（如 Vue SPA）。
//! 支持多个 SPA 项目，每个项目配置独立的 prefix 和 SPA 模式。
//!
//! ## 配置示例 (config.toml)
//!
//! ### 单个 SPA
//! ```toml
//! [serve]
//! prefix = "/"           # URL 前缀, 默认 "/"
//! spa = true             # SPA 模式: 未找到文件时返回 index.html
//! ```
//!
//! ### 多个 SPA 项目
//! ```toml
//! [[serve]]
//! prefix = "/admin"
//! spa = true
//!
//! [[serve]]
//! prefix = "/app"
//! spa = true
//!
//! [[serve]]
//! prefix = "/docs"
//! spa = false
//! ```
//!
//! ## 代码示例
//!
//! ```rust,no_run
//! // 运行时目录模式（单个）
//! let app = AFaster::new("config.toml".into()).await?
//!     .with_serve(afaster::serve::Serve::from_dir("./dist"))
//!     .service(svc)
//!     .run().await;
//!
//! // 多个 SPA 项目
//! let app = AFaster::new("config.toml".into()).await?
//!     .with_serves(vec![
//!         afaster::serve::Serve::from_dir("./admin/dist").with_prefix("/admin").with_spa(true),
//!         afaster::serve::Serve::from_dir("./app/dist").with_prefix("/app").with_spa(true),
//!     ])
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
        let dir = dir.into();
        let dir = dir.canonicalize().unwrap_or(dir);
        Self {
            prefix: "/".to_string(),
            spa: false,
            source: ServeSource::Dir(dir),
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
        let dir = PathBuf::from(default_dir);
        let dir = dir.canonicalize().unwrap_or(dir);
        Self {
            prefix: config.prefix.clone().unwrap_or_else(|| "/".to_string()),
            spa: config.spa,
            source: ServeSource::Dir(dir),
        }
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Vec<Self>> {
        // 尝试解析 [[serve]] 数组形式
        if let Some(serves) = table.get("serve") {
            // 判断是数组还是单个 table
            if let Ok(arr) = serves.clone().try_into::<Vec<ServeConfig>>() {
                return arr
                    .into_iter()
                    .map(|c| Ok(Self::from_config(&c, "./static")))
                    .collect();
            }
            // 单个 [serve] section，兼容旧配置
            if let Ok(config) = serves.clone().try_into::<ServeConfig>() {
                return Ok(vec![Self::from_config(&config, "./static")]);
            }
        }
        Ok(Vec::new())
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

        // 安全检查: 防止路径遍历 — 拒绝任何包含 `..` 的 segment
        if path
            .split('/')
            .any(|seg| seg == ".." || seg == "%2e%2e" || seg == ".%2e" || seg == "%2e.")
        {
            return None;
        }

        let mime = mime_type(&path);

        match &self.source {
            ServeSource::Dir(dir) => {
                let full_path = dir.join(&path);
                // canonicalize 后检查是否在 root 下
                match full_path.canonicalize() {
                    Ok(canon) => {
                        if !canon.starts_with(dir) {
                            return None;
                        }
                        match std::fs::read(&canon) {
                            Ok(data) => Some((data, mime)),
                            Err(_) => None,
                        }
                    }
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
/// 匹配逻辑：按注册顺序遍历所有 Serve，找到 prefix 匹配的第一个。
#[afast::get(desc("Serve static files"), no_trace)]
pub async fn serve_handler(
    afast::FullPath(path): afast::FullPath,
    afast::State(state): afast::State<crate::AppState>,
) -> afast::HttpResult<afast::Serve> {
    let serves = &state.serve;
    if serves.is_empty() {
        return Err(afast::Error::custom(500, "Serve module not configured"));
    }

    for serve in serves {
        if path_matches_prefix(&path, &serve.prefix) {
            if let Some((data, content_type)) = serve.resolve(&path) {
                return Ok(afast::Serve { data, content_type });
            }
        }
    }

    Err(afast::Error::custom(404, "File not found"))
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

/// 判断请求路径是否匹配 Serve 的 prefix
///
/// 匹配规则：
/// - prefix 为 "/" 时匹配所有路径
/// - 否则检查路径是否以 prefix 开头，且 prefix 后紧跟 `/`、`?` 或字符串结束
fn path_matches_prefix(path: &str, prefix: &str) -> bool {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() || prefix == "/" {
        return true;
    }

    let prefix = prefix.trim_start_matches('/');
    let path_trimmed = path.trim_start_matches('/');

    if let Some(rest) = path_trimmed.strip_prefix(prefix) {
        rest.is_empty() || rest.starts_with('/') || rest.starts_with('?')
    } else {
        false
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
    /// 设置静态文件服务（单个）
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

    /// 批量设置静态文件服务（多个 SPA 项目）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// AFaster::new("config.toml".into()).await?
    ///     .with_serves(vec![
    ///         afaster::serve::Serve::from_dir("./admin/dist").with_prefix("/admin").with_spa(true),
    ///         afaster::serve::Serve::from_dir("./app/dist").with_prefix("/app").with_spa(true),
    ///     ])
    ///     .run().await;
    /// ```
    fn with_serves(self, serves: Vec<Serve>) -> Self;
}

impl AFasterServeExt for crate::AFaster {
    fn with_serve(mut self, serve: Serve) -> Self {
        self.state.serve.push(serve);
        self
    }

    fn with_serves(mut self, serves: Vec<Serve>) -> Self {
        self.state.serve.extend(serves);
        self
    }
}
