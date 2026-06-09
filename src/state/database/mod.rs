pub mod err;
use err::*;

use serde::Deserialize;

#[cfg(feature = "db-postgres")]
pub use sqlx::postgres::PgPool;

#[cfg(feature = "db-sqlite")]
pub use sqlx::sqlite::SqlitePool;

#[cfg(feature = "db-mysql")]
pub use sqlx::mysql::MySqlPool;

// ═══════════════════════════════════════════════════════════════
//  数据库配置
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "db-postgres")]
#[derive(Clone, Deserialize)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub name: String,
}

#[cfg(feature = "db-sqlite")]
#[derive(Clone, Deserialize)]
pub struct SqliteConfig {
    pub path: String,
}

#[cfg(feature = "db-mysql")]
#[derive(Clone, Deserialize)]
pub struct MysqlConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub name: String,
}

// ═══════════════════════════════════════════════════════════════
//  数据库连接池
// ═══════════════════════════════════════════════════════════════

/// 数据库连接池
///
/// 根据启用的 feature 自动选择 PostgreSQL / SQLite / MySQL。
/// 三种数据库互斥，同时只能启用一个。
#[derive(Clone)]
pub struct Database {
    #[cfg(feature = "db-postgres")]
    pub pool: PgPool,
    #[cfg(feature = "db-sqlite")]
    pub pool: SqlitePool,
    #[cfg(feature = "db-mysql")]
    pub pool: MySqlPool,
}

impl Database {
    /// 从配置建立数据库连接
    #[cfg(feature = "db-postgres")]
    pub async fn connect_postgres(config: &PostgresConfig) -> crate::Result<Self> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.user, config.pass, config.host, config.port, config.name
        );
        let pool = PgPool::connect(&url).await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "PostgreSQL: {}", e);
            connect_failed("PostgreSQL")
        })?;
        Ok(Self { pool })
    }

    /// 从配置建立数据库连接
    #[cfg(feature = "db-sqlite")]
    pub async fn connect_sqlite(config: &SqliteConfig) -> crate::Result<Self> {
        let url = if config.path.is_empty() {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{}?mode=rwc", config.path)
        };
        let pool = SqlitePool::connect(&url).await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "SQLite: {}", e);
            connect_failed("SQLite")
        })?;
        Ok(Self { pool })
    }

    /// 从配置建立数据库连接
    #[cfg(feature = "db-mysql")]
    pub async fn connect_mysql(config: &MysqlConfig) -> crate::Result<Self> {
        let url = format!(
            "mysql://{}:{}@{}:{}/{}",
            config.user, config.pass, config.host, config.port, config.name
        );
        let pool = MySqlPool::connect(&url).await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "MySQL: {}", e);
            connect_failed("MySQL")
        })?;
        Ok(Self { pool })
    }
}
