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
/// 支持同时启用多种数据库。每种数据库通过独立字段持有连接池，
/// 可通过 `pg()` / `sqlite()` / `mysql()` 方法获取对应池引用。
#[derive(Clone)]
pub struct Database {
    #[cfg(feature = "db-postgres")]
    pub pg: PgPool,
    #[cfg(feature = "db-sqlite")]
    pub sqlite: SqlitePool,
    #[cfg(feature = "db-mysql")]
    pub mysql: MySqlPool,
}

impl Database {
    /// 获取 PostgreSQL 连接池引用
    #[cfg(feature = "db-postgres")]
    pub fn pg(&self) -> &PgPool {
        &self.pg
    }

    /// 获取 SQLite 连接池引用
    #[cfg(feature = "db-sqlite")]
    pub fn sqlite(&self) -> &SqlitePool {
        &self.sqlite
    }

    /// 获取 MySQL 连接池引用
    #[cfg(feature = "db-mysql")]
    pub fn mysql(&self) -> &MySqlPool {
        &self.mysql
    }

    /// 获取默认连接池引用（仅启用单个数据库时可用）
    ///
    /// - 仅启用 `db-postgres` → 返回 `&PgPool`
    /// - 仅启用 `db-sqlite` → 返回 `&SqlitePool`
    /// - 仅启用 `db-mysql` → 返回 `&MySqlPool`
    /// - 启用多个 → 编译错误，请使用 `pg()` / `sqlite()` / `mysql()`
    #[cfg(feature = "db-postgres")]
    #[cfg(not(any(feature = "db-sqlite", feature = "db-mysql")))]
    pub fn pool(&self) -> &PgPool {
        &self.pg
    }

    #[cfg(feature = "db-sqlite")]
    #[cfg(not(any(feature = "db-postgres", feature = "db-mysql")))]
    pub fn pool(&self) -> &SqlitePool {
        &self.sqlite
    }

    #[cfg(feature = "db-mysql")]
    #[cfg(not(any(feature = "db-postgres", feature = "db-sqlite")))]
    pub fn pool(&self) -> &MySqlPool {
        &self.mysql
    }

    /// 建立数据库连接（根据启用的 feature 连接所有配置的数据库）
    pub async fn connect(
        #[cfg(feature = "db-postgres")] postgres: &PostgresConfig,
        #[cfg(feature = "db-sqlite")] sqlite: &SqliteConfig,
        #[cfg(feature = "db-mysql")] mysql: &MysqlConfig,
    ) -> crate::Result<Self> {
        #[cfg(feature = "db-postgres")]
        let pg = Self::connect_postgres(postgres).await?;

        #[cfg(feature = "db-sqlite")]
        let sqlite = Self::connect_sqlite(sqlite).await?;

        #[cfg(feature = "db-mysql")]
        let mysql = Self::connect_mysql(mysql).await?;

        Ok(Self {
            #[cfg(feature = "db-postgres")]
            pg,
            #[cfg(feature = "db-sqlite")]
            sqlite,
            #[cfg(feature = "db-mysql")]
            mysql,
        })
    }

    /// 建立 PostgreSQL 连接池
    #[cfg(feature = "db-postgres")]
    async fn connect_postgres(config: &PostgresConfig) -> crate::Result<PgPool> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.user, config.pass, config.host, config.port, config.name
        );
        PgPool::connect(&url).await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "PostgreSQL: {}", _e);
            connect_failed("PostgreSQL")
        })
    }

    /// 建立 SQLite 连接池
    #[cfg(feature = "db-sqlite")]
    async fn connect_sqlite(config: &SqliteConfig) -> crate::Result<SqlitePool> {
        let url = if config.path.is_empty() {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite:{}?mode=rwc", config.path)
        };
        SqlitePool::connect(&url).await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "SQLite: {}", _e);
            connect_failed("SQLite")
        })
    }

    /// 建立 MySQL 连接池
    #[cfg(feature = "db-mysql")]
    async fn connect_mysql(config: &MysqlConfig) -> crate::Result<MySqlPool> {
        let url = format!(
            "mysql://{}:{}@{}:{}/{}",
            config.user, config.pass, config.host, config.port, config.name
        );
        MySqlPool::connect(&url).await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "MySQL: {}", _e);
            connect_failed("MySQL")
        })
    }
}
