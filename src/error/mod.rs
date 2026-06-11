// ═══════════════════════════════════════════════════════════════
//  afaster 统一错误类型
//
//  所有模块返回 afaster::Error，handler 层自动转换为 afast::Error。
//  支持 From<sqlx::Error>、From<reqwest::Error> 等，可以用 `?` 直接传播。
// ═══════════════════════════════════════════════════════════════

/// afaster 统一 Result 类型
pub type Result<T> = std::result::Result<T, Error>;

/// afaster 统一错误类型
#[derive(Debug)]
pub enum Error {
    /// 自定义业务错误（code + message）
    Custom { code: i64, message: String },
    /// 数据库错误
    #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
    Database(sqlx::Error),
    /// HTTP 请求错误
    #[cfg(any(
        feature = "wx-login-mini",
        feature = "wx-login-app",
        feature = "wx-login-web",
        feature = "wx-official",
        feature = "oss",
        feature = "cos",
        feature = "wx-virtual-pay",
        feature = "github-oauth2",
        feature = "wx-sec-check",
        feature = "wx-pay-h5",
        feature = "sms-ali",
        feature = "sms-tencent",
        feature = "amap",
        feature = "tmap",
        feature = "push-getui",
        feature = "push-jpush",
        feature = "push-xiaomi",
        feature = "ali-pay-web"
    ))]
    Http(reqwest::Error),
    /// JSON 序列化/反序列化错误
    #[cfg(any(
        feature = "wx-login-mini",
        feature = "wx-login-app",
        feature = "wx-login-web",
        feature = "wx-official",
        feature = "oss",
        feature = "cos",
        feature = "wx-virtual-pay",
        feature = "github-oauth2",
        feature = "wx-sec-check",
        feature = "wx-pay-h5",
        feature = "sms-ali",
        feature = "sms-tencent",
        feature = "push-getui",
        feature = "push-jpush",
        feature = "push-xiaomi",
        feature = "sse",
        feature = "excel",
        feature = "ali-pay-web"
    ))]
    Json(serde_json::Error),
    /// IO 错误
    Io(std::io::Error),
    /// 通用字符串错误
    Other(String),
}

impl Error {
    /// 创建自定义业务错误
    #[inline]
    pub fn custom(code: i64, message: impl Into<String>) -> Self {
        Error::Custom {
            code,
            message: message.into(),
        }
    }

    /// 获取错误码
    pub fn code(&self) -> i64 {
        match self {
            Error::Custom { code, .. } => *code,
            #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
            Error::Database(_) => 50002,
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web",
                feature = "wx-official",
                feature = "oss",
                feature = "cos",
                feature = "wx-virtual-pay",
                feature = "github-oauth2",
                feature = "wx-sec-check",
                feature = "wx-pay-h5",
                feature = "sms-ali",
                feature = "sms-tencent",
                feature = "amap",
                feature = "tmap",
                feature = "push-getui",
                feature = "push-jpush",
                feature = "push-xiaomi",
                feature = "ali-pay-web"
            ))]
            Error::Http(_) => 50003,
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web",
                feature = "wx-official",
                feature = "oss",
                feature = "cos",
                feature = "wx-virtual-pay",
                feature = "github-oauth2",
                feature = "wx-sec-check",
                feature = "wx-pay-h5",
                feature = "sms-ali",
                feature = "sms-tencent",
                feature = "push-getui",
                feature = "push-jpush",
                feature = "push-xiaomi",
                feature = "sse",
                feature = "excel",
                feature = "ali-pay-web",
            ))]
            Error::Json(_) => 50004,
            Error::Io(_) => 50005,
            Error::Other(_) => 50001,
        }
    }

    /// 获取错误消息
    pub fn message(&self) -> String {
        match self {
            Error::Custom { message, .. } => message.clone(),
            #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
            Error::Database(e) => format!("Database error: {}", e),
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web",
                feature = "wx-official",
                feature = "oss",
                feature = "cos",
                feature = "wx-virtual-pay",
                feature = "github-oauth2",
                feature = "wx-sec-check",
                feature = "wx-pay-h5",
                feature = "sms-ali",
                feature = "sms-tencent",
                feature = "amap",
                feature = "tmap",
                feature = "push-getui",
                feature = "push-jpush",
                feature = "push-xiaomi",
                feature = "ali-pay-web"
            ))]
            Error::Http(e) => format!("HTTP error: {}", e),
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web",
                feature = "wx-official",
                feature = "oss",
                feature = "cos",
                feature = "wx-virtual-pay",
                feature = "github-oauth2",
                feature = "wx-sec-check",
                feature = "wx-pay-h5",
                feature = "sms-ali",
                feature = "sms-tencent",
                feature = "push-getui",
                feature = "push-jpush",
                feature = "push-xiaomi",
                feature = "sse",
                feature = "excel",
                feature = "ali-pay-web",
            ))]
            Error::Json(e) => format!("JSON error: {}", e),
            Error::Io(e) => format!("IO error: {}", e),
            Error::Other(s) => s.clone(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.message())
    }
}

impl std::error::Error for Error {}

/// 实现 afast 的 AFastError trait，使 handler 可以直接返回 crate::Result<T>
impl afast::AFastError for Error {
    fn code(&self) -> i64 {
        Error::code(self)
    }

    fn message(&self) -> String {
        Error::message(self)
    }

    fn into_error(self) -> afast::Error {
        afast::Error::custom(self.code(), self.message())
    }
}

// ── From 实现：支持 `?` 自动传播 ──────────────────────────────

/// afaster::Error → afast::Error（handler 返回值自动转换）
impl From<Error> for afast::Error {
    fn from(e: Error) -> Self {
        afast::Error::custom(e.code(), e.message())
    }
}

/// sqlx::Error → afaster::Error（数据库操作直接用 `?`）
#[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        #[cfg(feature = "log")]
        tracing::error!("Database error: {:?}", e);
        Error::Database(e)
    }
}

/// reqwest::Error → afaster::Error（HTTP 请求直接用 `?`）
#[cfg(any(
    feature = "wx-login-mini",
    feature = "wx-login-app",
    feature = "wx-login-web",
    feature = "wx-official",
    feature = "oss",
    feature = "cos",
    feature = "wx-virtual-pay",
    feature = "github-oauth2",
    feature = "wx-sec-check",
    feature = "wx-pay-h5",
    feature = "sms-ali",
    feature = "sms-tencent",
    feature = "amap",
    feature = "tmap",
    feature = "push-getui",
    feature = "push-jpush",
    feature = "push-xiaomi",
    feature = "ali-pay-web"
))]
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

/// serde_json::Error → afaster::Error
#[cfg(any(
    feature = "wx-login-mini",
    feature = "wx-login-app",
    feature = "wx-login-web",
    feature = "wx-official",
    feature = "oss",
    feature = "cos",
    feature = "wx-virtual-pay",
    feature = "github-oauth2",
    feature = "wx-sec-check",
    feature = "wx-pay-h5",
    feature = "sms-ali",
    feature = "sms-tencent",
    feature = "push-getui",
    feature = "push-jpush",
    feature = "push-xiaomi",
    feature = "sse",
    feature = "excel",
    feature = "ali-pay-web"
))]
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

/// std::io::Error → afaster::Error
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// afast::Error → afaster::Error（方便在模块内传播 afast 错误）
impl From<afast::Error> for Error {
    fn from(e: afast::Error) -> Self {
        Error::Custom {
            code: e.code(),
            message: e.message().to_string(),
        }
    }
}

// ── 兼容旧代码的辅助函数 ──────────────────────────────────────

/// 服务端内部错误 (50001)
#[allow(dead_code)]
#[inline]
pub fn server() -> Error {
    Error::custom(50001, "Internal server error")
}
