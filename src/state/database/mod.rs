pub mod err;
use err::*;

use serde::Deserialize;

/// 对数据库密码中的特殊字符进行 percent-encoding
/// RFC 3986: userinfo 中的 ':' 以及 '@' '/' '?' '#' 等必须编码
#[cfg(any(feature = "db-postgres", feature = "db-mysql"))]
fn encode_db_password(password: &str) -> String {
    let mut out = String::with_capacity(password.len() * 3);
    for &b in password.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

#[cfg(feature = "db-ahrisqle")]
pub use ahrisql::pool::embedded::EmbeddedPool;

#[cfg(feature = "db-ahrisqls")]
pub use ahrisql::pool::Pool;

#[cfg(feature = "db-postgres")]
pub use sqlx::postgres::PgPool;

#[cfg(feature = "db-sqlite")]
pub use sqlx::sqlite::SqlitePool;

#[cfg(feature = "db-mysql")]
pub use sqlx::mysql::MySqlPool;

// ═══════════════════════════════════════════════════════════════
//  数据库配置
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "db-ahrisqle")]
#[derive(Clone, Deserialize)]
pub struct AhriSqleConfig {
    pub path: String,
    pub user: String,
    pub pass: String,
    pub name: String,
}

#[cfg(feature = "db-ahrisqls")]
#[derive(Clone, Deserialize)]
pub struct AhriSqlsConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub name: String,
}

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
/// 可通过 `ahrisqle()` / `ahrisqls()` / pg()` / `sqlite()` / `mysql()` 方法获取对应池引用。
#[derive(Clone)]
pub struct Database {
    #[cfg(feature = "db-ahrisqle")]
    pub ahrisqle: EmbeddedPool,
    #[cfg(feature = "db-ahrisqls")]
    pub ahrisqls: Pool,
    #[cfg(feature = "db-postgres")]
    pub pg: PgPool,
    #[cfg(feature = "db-sqlite")]
    pub sqlite: SqlitePool,
    #[cfg(feature = "db-mysql")]
    pub mysql: MySqlPool,
}

impl Database {
    /// 获取嵌入式 AhriSQL 连接池引用
    #[cfg(feature = "db-ahrisqle")]
    pub fn ahrisqle(&self) -> &EmbeddedPool {
        &self.ahrisqle
    }

    /// 获取 AhriSQL 连接池引用
    #[cfg(feature = "db-ahrisqls")]
    pub fn ahrisqls(&self) -> &Pool {
        &self.ahrisqls
    }

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
    /// - 仅启用 `db-ahrisqle` → 返回 `&EmbeddedPool`
    /// - 仅启用 `db-ahrisqls` → 返回 `&Pool`
    /// - 仅启用 `db-postgres` → 返回 `&PgPool`
    /// - 仅启用 `db-sqlite` → 返回 `&SqlitePool`
    /// - 仅启用 `db-mysql` → 返回 `&MySqlPool`
    /// - 启用多个 → 编译错误，请使用 `pg()` / `sqlite()` / `mysql()`
    #[cfg(feature = "db-ahrisqle")]
    #[cfg(not(any(
        feature = "db-ahrisqls",
        feature = "db-postgres",
        feature = "db-sqlite",
        feature = "db-mysql"
    )))]
    pub fn pool(&self) -> &EmbeddedPool {
        &self.ahrisqle
    }

    #[cfg(feature = "db-ahrisqls")]
    #[cfg(not(any(
        feature = "db-ahrisqle",
        feature = "db-postgres",
        feature = "db-sqlite",
        feature = "db-mysql"
    )))]
    pub fn pool(&self) -> &Pool {
        &self.ahrisqls
    }

    #[cfg(feature = "db-postgres")]
    #[cfg(not(any(
        feature = "db-ahrisqle",
        feature = "db-ahrisqls",
        feature = "db-sqlite",
        feature = "db-mysql"
    )))]
    pub fn pool(&self) -> &PgPool {
        &self.pg
    }

    #[cfg(feature = "db-sqlite")]
    #[cfg(not(any(
        feature = "db-ahrisqle",
        feature = "db-ahrisqls",
        feature = "db-postgres",
        feature = "db-mysql"
    )))]
    pub fn pool(&self) -> &SqlitePool {
        &self.sqlite
    }

    #[cfg(feature = "db-mysql")]
    #[cfg(not(any(
        feature = "db-ahrisqle",
        feature = "db-ahrisqls",
        feature = "db-postgres",
        feature = "db-sqlite"
    )))]
    pub fn pool(&self) -> &MySqlPool {
        &self.mysql
    }

    /// 建立数据库连接（根据启用的 feature 连接所有配置的数据库）
    pub async fn connect(
        #[cfg(feature = "db-ahrisqle")] ahrisqle: &AhriSqleConfig,
        #[cfg(feature = "db-ahrisqls")] ahrisqls: &AhriSqlsConfig,
        #[cfg(feature = "db-postgres")] postgres: &PostgresConfig,
        #[cfg(feature = "db-sqlite")] sqlite: &SqliteConfig,
        #[cfg(feature = "db-mysql")] mysql: &MysqlConfig,
    ) -> crate::Result<Self> {
        #[cfg(feature = "db-ahrisqle")]
        let ahrisqle = Self::connect_ahrisqle(ahrisqle).await?;

        #[cfg(feature = "db-ahrisqls")]
        let ahrisqls = Self::connect_ahrisqls(ahrisqls).await?;

        #[cfg(feature = "db-postgres")]
        let pg = Self::connect_postgres(postgres).await?;

        #[cfg(feature = "db-sqlite")]
        let sqlite = Self::connect_sqlite(sqlite).await?;

        #[cfg(feature = "db-mysql")]
        let mysql = Self::connect_mysql(mysql).await?;

        Ok(Self {
            #[cfg(feature = "db-ahrisqle")]
            ahrisqle,
            #[cfg(feature = "db-ahrisqls")]
            ahrisqls,
            #[cfg(feature = "db-postgres")]
            pg,
            #[cfg(feature = "db-sqlite")]
            sqlite,
            #[cfg(feature = "db-mysql")]
            mysql,
        })
    }

    pub async fn from_table(table: &toml::Table) -> crate::Result<Self> {
        #[cfg(feature = "db-ahrisqls")]
        let ahrisqls: AhriSqlsConfig = crate::state::extract(table, "ahrisqls")?;
        #[cfg(feature = "db-postgres")]
        let postgres: PostgresConfig = crate::state::extract(table, "postgres")?;
        #[cfg(feature = "db-sqlite")]
        let sqlite: SqliteConfig = crate::state::extract(table, "sqlite")?;
        #[cfg(feature = "db-mysql")]
        let mysql: MysqlConfig = crate::state::extract(table, "mysql")?;

        Self::connect(
            #[cfg(feature = "db-ahrisqle")]
            &ahrisqle,
            #[cfg(feature = "db-ahrisqls")]
            &ahrisqls,
            #[cfg(feature = "db-postgres")]
            &postgres,
            #[cfg(feature = "db-sqlite")]
            &sqlite,
            #[cfg(feature = "db-mysql")]
            &mysql,
        )
        .await
    }

    /// 建立 AhriSQL 连接池
    #[cfg(feature = "db-ahrisqle")]
    async fn connect_ahrisqle(config: &AhriSqleConfig) -> crate::Result<EmbeddedPool> {
        EmbeddedPool::open(&config.path, &config.name, &config.user, &config.pass)
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50901, msg = "Database open failed" }, "AhriSQL: {}", _e);
                connect_failed("AhriSQL")
            })
    }

    /// 建立 AhriSQL 连接池
    #[cfg(feature = "db-ahrisqls")]
    async fn connect_ahrisqls(config: &AhriSqlsConfig) -> crate::Result<Pool> {
        let pool = Pool::builder()
            .host(&config.host)
            .port(config.port)
            .user(&config.user)
            .password(&config.pass)
            .database(&config.name)
            .max_connections(64)
            .build();
        Ok(pool)
    }

    /// 建立 PostgreSQL 连接池
    #[cfg(feature = "db-postgres")]
    async fn connect_postgres(config: &PostgresConfig) -> crate::Result<PgPool> {
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            config.user,
            encode_db_password(&config.pass),
            config.host,
            config.port,
            config.name
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
            tracing::error!({ code = 50901, msg = "Database open failed" }, "SQLite: {}", _e);
            connect_failed("SQLite")
        })
    }

    /// 建立 MySQL 连接池
    #[cfg(feature = "db-mysql")]
    async fn connect_mysql(config: &MysqlConfig) -> crate::Result<MySqlPool> {
        let url = format!(
            "mysql://{}:{}@{}:{}/{}",
            config.user,
            encode_db_password(&config.pass),
            config.host,
            config.port,
            config.name
        );
        MySqlPool::connect(&url).await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "MySQL: {}", _e);
            connect_failed("MySQL")
        })
    }
}
