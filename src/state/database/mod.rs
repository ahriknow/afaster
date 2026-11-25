pub mod err;
use err::*;

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
#[derive(Clone)]
pub struct PostgresConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    pub name: String,
}

#[cfg(feature = "db-sqlite")]
#[derive(Clone)]
pub struct SqliteConfig {
    pub path: String,
}

#[cfg(feature = "db-mysql")]
#[derive(Clone)]
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
/// 可通过 pg()` / `sqlite()` / `mysql()` 方法获取对应池引用。
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

    pub async fn from_table(table: &toml::Table) -> crate::Result<Self> {
        #[cfg(feature = "db-postgres")]
        let postgres = {
            let map = table
                .get("postgres")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            PostgresConfig {
                host: match (map.get("host"), std::env::var("POSTGRES_HOST")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[postgres].host must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => "127.0.0.1".to_owned(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(
                            50001,
                            "POSTGRES_HOST is not valid UTF-8",
                        ));
                    }
                },
                port: match (map.get("port"), std::env::var("POSTGRES_PORT")) {
                    (_, Ok(value)) => value.parse().map_err(|error| {
                        crate::Error::custom(50001, format!("Invalid POSTGRES_PORT: {error}"))
                    })?,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_integer()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[postgres].port must be an integer")
                        })?
                        .try_into()
                        .map_err(|_| {
                            crate::Error::custom(50001, "[postgres].port is out of range")
                        })?,
                    (None, Err(std::env::VarError::NotPresent)) => 5432,
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(
                            50001,
                            "POSTGRES_PORT is not valid UTF-8",
                        ));
                    }
                },
                user: match (map.get("user"), std::env::var("POSTGRES_USER")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[postgres].user must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => "postgres".to_owned(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(
                            50001,
                            "POSTGRES_USER is not valid UTF-8",
                        ));
                    }
                },
                pass: match (map.get("pass"), std::env::var("POSTGRES_PASS")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[postgres].pass must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => String::new(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(
                            50001,
                            "POSTGRES_PASS is not valid UTF-8",
                        ));
                    }
                },
                name: match (map.get("name"), std::env::var("POSTGRES_NAME")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[postgres].name must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => "postgres".to_owned(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(
                            50001,
                            "POSTGRES_NAME is not valid UTF-8",
                        ));
                    }
                },
            }
        };
        #[cfg(feature = "db-sqlite")]
        let sqlite = {
            let map = table
                .get("sqlite")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            SqliteConfig {
                path: match (map.get("path"), std::env::var("SQLITE_PATH")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[sqlite].path must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => String::new(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(
                            50001,
                            "SQLITE_PATH is not valid UTF-8",
                        ));
                    }
                },
            }
        };
        #[cfg(feature = "db-mysql")]
        let mysql = {
            let map = table
                .get("mysql")
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            MysqlConfig {
                host: match (map.get("host"), std::env::var("MYSQL_HOST")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[mysql].host must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => "127.0.0.1".to_owned(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(50001, "MYSQL_HOST is not valid UTF-8"));
                    }
                },
                port: match (map.get("port"), std::env::var("MYSQL_PORT")) {
                    (_, Ok(value)) => value.parse().map_err(|error| {
                        crate::Error::custom(50001, format!("Invalid MYSQL_PORT: {error}"))
                    })?,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_integer()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[mysql].port must be an integer")
                        })?
                        .try_into()
                        .map_err(|_| crate::Error::custom(50001, "[mysql].port is out of range"))?,
                    (None, Err(std::env::VarError::NotPresent)) => 3306,
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(50001, "MYSQL_PORT is not valid UTF-8"));
                    }
                },
                user: match (map.get("user"), std::env::var("MYSQL_USER")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[mysql].user must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => "root".to_owned(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(50001, "MYSQL_USER is not valid UTF-8"));
                    }
                },
                pass: match (map.get("pass"), std::env::var("MYSQL_PASS")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[mysql].pass must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => String::new(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(50001, "MYSQL_PASS is not valid UTF-8"));
                    }
                },
                name: match (map.get("name"), std::env::var("MYSQL_NAME")) {
                    (_, Ok(value)) => value,
                    (Some(value), Err(std::env::VarError::NotPresent)) => value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(50001, "[mysql].name must be a string")
                        })?
                        .to_owned(),
                    (None, Err(std::env::VarError::NotPresent)) => "mysql".to_owned(),
                    (_, Err(std::env::VarError::NotUnicode(_))) => {
                        return Err(crate::Error::custom(50001, "MYSQL_NAME is not valid UTF-8"));
                    }
                },
            }
        };

        Self::connect(
            #[cfg(feature = "db-postgres")]
            &postgres,
            #[cfg(feature = "db-sqlite")]
            &sqlite,
            #[cfg(feature = "db-mysql")]
            &mysql,
        )
        .await
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
        PgPool::connect_lazy(&url).map_err(|_e| {
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
        SqlitePool::connect_lazy(&url).map_err(|_e| {
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
        MySqlPool::connect_lazy(&url).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50901, msg = "Database connection failed" }, "MySQL: {}", _e);
            connect_failed("MySQL")
        })
    }
}
