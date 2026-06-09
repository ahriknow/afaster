pub mod err;
use err::*;

use redis::AsyncCommands;
use serde::Deserialize;

// ═══════════════════════════════════════════════════════════════
//  Redis / Valkey 配置
// ═══════════════════════════════════════════════════════════════

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    6379
}

fn default_db() -> i64 {
    0
}

/// Redis / Valkey 连接配置
///
/// 从 config.toml 的 `[redis]` 或 `[valkey]` 节反序列化。
#[derive(Clone, Deserialize)]
pub struct RedisConfig {
    /// 主机地址（可选，默认 127.0.0.1）
    #[serde(default = "default_host")]
    pub host: String,
    /// 端口（可选，默认 6379）
    #[serde(default = "default_port")]
    pub port: u16,
    /// 数据库编号（可选，默认 0）
    #[serde(default = "default_db")]
    pub db: i64,
    /// 密码（可选），为空字符串时不使用密码连接
    #[serde(default)]
    pub password: String,
    /// Key 前缀（可选），所有 key 自动添加前缀，用于多租户隔离
    #[serde(default)]
    pub prefix: String,
}

// ═══════════════════════════════════════════════════════════════
//  Redis / Valkey 客户端
// ═══════════════════════════════════════════════════════════════

/// Redis / Valkey 客户端
///
/// 基于 `redis` crate，支持 Redis 和 Valkey（协议完全兼容）。
///
/// ```no_run
/// let redis = Redis::connect(&config).await?;
/// redis.set("key", "value", Some(3600)).await?;
/// let val: Option<String> = redis.get("key").await?;
/// ```
#[derive(Clone)]
pub struct Redis {
    client: redis::Client,
    db: i64,
    prefix: String,
}

impl Redis {
    /// 从配置建立连接
    pub async fn connect(config: &RedisConfig) -> crate::Result<Self> {
        let conn_info = redis::ConnectionInfo {
            addr: redis::ConnectionAddr::Tcp(config.host.clone(), config.port),
            redis: redis::RedisConnectionInfo {
                db: config.db,
                password: if config.password.is_empty() {
                    None
                } else {
                    Some(config.password.clone())
                },
                username: None,
                protocol: redis::ProtocolVersion::RESP2,
            },
        };

        let client = redis::Client::open(conn_info).map_err(|e| connect_failed(&e.to_string()))?;

        // 验证连接
        let mut conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| connect_failed(&e.to_string()))?;

        // 选择数据库（ConnectionInfo 已设置 db，这里再确认一次）
        if config.db != 0 {
            redis::cmd("SELECT")
                .arg(config.db)
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| connect_failed(&e.to_string()))?;
        }

        Ok(Self {
            client,
            db: config.db,
            prefix: config.prefix.clone(),
        })
    }

    /// 获取异步连接
    async fn conn(&self) -> crate::Result<redis::aio::MultiplexedConnection> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| connection_lost(&e.to_string()))?;

        if self.db != 0 {
            redis::cmd("SELECT")
                .arg(self.db)
                .query_async::<()>(&mut conn)
                .await
                .map_err(|e| connection_lost(&e.to_string()))?;
        }

        Ok(conn)
    }

    /// 给 key 添加前缀
    fn key(&self, key: &str) -> String {
        if self.prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}:{}", self.prefix, key)
        }
    }

    // ──────────────────────────────────────────────────────────
    //  基础操作
    // ──────────────────────────────────────────────────────────

    /// GET — 获取值
    pub async fn get<V: redis::FromRedisValue>(&self, key: &str) -> crate::Result<Option<V>> {
        let mut conn = self.conn().await?;
        conn.get(self.key(key))
            .await
            .map_err(|e| cmd_failed("GET", &e.to_string()))
    }

    /// SET — 设置值
    ///
    /// - `ttl` 为过期时间（秒），`None` 表示永不过期
    pub async fn set<V: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: &str,
        value: V,
        ttl: Option<u64>,
    ) -> crate::Result<()> {
        let mut conn = self.conn().await?;
        let k = self.key(key);

        if let Some(seconds) = ttl {
            conn.set_ex(k, value, seconds)
                .await
                .map_err(|e| cmd_failed("SET", &e.to_string()))
        } else {
            conn.set(k, value)
                .await
                .map_err(|e| cmd_failed("SET", &e.to_string()))
        }
    }

    /// SET NX — 仅当 key 不存在时设置（用于分布式锁）
    pub async fn set_nx<V: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: &str,
        value: V,
        ttl: Option<u64>,
    ) -> crate::Result<bool> {
        let mut conn = self.conn().await?;
        let k = self.key(key);

        let result: bool = if let Some(seconds) = ttl {
            redis::cmd("SET")
                .arg(&k)
                .arg(value)
                .arg("EX")
                .arg(seconds)
                .arg("NX")
                .query_async(&mut conn)
                .await
                .map_err(|e| cmd_failed("SET NX", &e.to_string()))?
        } else {
            conn.set_nx(k, value)
                .await
                .map_err(|e| cmd_failed("SET NX", &e.to_string()))?
        };

        Ok(result)
    }

    /// DEL — 删除 key
    pub async fn del(&self, key: &str) -> crate::Result<bool> {
        let mut conn = self.conn().await?;
        let count: i64 = conn
            .del(self.key(key))
            .await
            .map_err(|e| cmd_failed("DEL", &e.to_string()))?;
        Ok(count > 0)
    }

    /// EXISTS — key 是否存在
    pub async fn exists(&self, key: &str) -> crate::Result<bool> {
        let mut conn = self.conn().await?;
        conn.exists(self.key(key))
            .await
            .map_err(|e| cmd_failed("EXISTS", &e.to_string()))
    }

    /// EXPIRE — 设置过期时间（秒）
    pub async fn expire(&self, key: &str, seconds: u64) -> crate::Result<bool> {
        let mut conn = self.conn().await?;
        conn.expire(self.key(key), seconds as i64)
            .await
            .map_err(|e| cmd_failed("EXPIRE", &e.to_string()))
    }

    /// TTL — 获取剩余过期时间（秒），-1 永不过期，-2 不存在
    pub async fn ttl(&self, key: &str) -> crate::Result<i64> {
        let mut conn = self.conn().await?;
        conn.ttl(self.key(key))
            .await
            .map_err(|e| cmd_failed("TTL", &e.to_string()))
    }

    /// KEYS — 按模式查找 key（慎用，生产环境建议用 SCAN）
    pub async fn keys(&self, pattern: &str) -> crate::Result<Vec<String>> {
        let mut conn = self.conn().await?;
        let p = if self.prefix.is_empty() {
            pattern.to_string()
        } else {
            format!("{}:{}", self.prefix, pattern)
        };
        conn.keys(p)
            .await
            .map_err(|e| cmd_failed("KEYS", &e.to_string()))
    }

    /// INCR — 原子自增
    pub async fn incr(&self, key: &str, delta: i64) -> crate::Result<i64> {
        let mut conn = self.conn().await?;
        conn.incr(self.key(key), delta)
            .await
            .map_err(|e| cmd_failed("INCR", &e.to_string()))
    }

    // ──────────────────────────────────────────────────────────
    //  Hash 操作
    // ──────────────────────────────────────────────────────────

    /// HGET — 获取 hash 字段值
    pub async fn hget<V: redis::FromRedisValue>(
        &self,
        key: &str,
        field: &str,
    ) -> crate::Result<Option<V>> {
        let mut conn = self.conn().await?;
        conn.hget(self.key(key), field)
            .await
            .map_err(|e| cmd_failed("HGET", &e.to_string()))
    }

    /// HSET — 设置 hash 字段
    pub async fn hset<V: redis::ToRedisArgs + Send + Sync>(
        &self,
        key: &str,
        field: &str,
        value: V,
    ) -> crate::Result<()> {
        let mut conn = self.conn().await?;
        conn.hset(self.key(key), field, value)
            .await
            .map_err(|e| cmd_failed("HSET", &e.to_string()))
    }

    /// HDEL — 删除 hash 字段
    pub async fn hdel(&self, key: &str, field: &str) -> crate::Result<bool> {
        let mut conn = self.conn().await?;
        let count: i64 = conn
            .hdel(self.key(key), field)
            .await
            .map_err(|e| cmd_failed("HDEL", &e.to_string()))?;
        Ok(count > 0)
    }

    /// HGETALL — 获取所有 hash 字段和值
    pub async fn hgetall(
        &self,
        key: &str,
    ) -> crate::Result<std::collections::HashMap<String, String>> {
        let mut conn = self.conn().await?;
        conn.hgetall(self.key(key))
            .await
            .map_err(|e| cmd_failed("HGETALL", &e.to_string()))
    }

    /// HINCRBY — hash 字段原子自增
    pub async fn hincrby(&self, key: &str, field: &str, delta: i64) -> crate::Result<i64> {
        let mut conn = self.conn().await?;
        conn.hincr(self.key(key), field, delta)
            .await
            .map_err(|e| cmd_failed("HINCRBY", &e.to_string()))
    }

    // ──────────────────────────────────────────────────────────
    //  分布式锁
    // ──────────────────────────────────────────────────────────

    /// 获取分布式锁
    ///
    /// - `lock_key` — 锁名
    /// - `lock_value` — 锁的标识（用于安全释放，建议用 UUID）
    /// - `ttl` — 锁的自动过期时间（秒）
    ///
    /// 返回 `true` 表示成功获取锁
    pub async fn lock(&self, lock_key: &str, lock_value: &str, ttl: u64) -> crate::Result<bool> {
        let key = format!("lock:{}", lock_key);
        self.set_nx(&key, lock_value, Some(ttl)).await
    }

    /// 释放分布式锁（仅当 value 匹配时释放，防止误释放他人锁）
    ///
    /// 使用 Lua 脚本保证原子性
    pub async fn unlock(&self, lock_key: &str, lock_value: &str) -> crate::Result<bool> {
        let mut conn = self.conn().await?;
        let key = self.key(&format!("lock:{}", lock_key));

        let script = redis::Script::new(
            r#"if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end"#,
        );

        let result: i64 = script
            .key(&key)
            .arg(lock_value)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| cmd_failed("UNLOCK", &e.to_string()))?;

        Ok(result > 0)
    }

    // ──────────────────────────────────────────────────────────
    //  发布/订阅（Pub/Sub）
    // ──────────────────────────────────────────────────────────

    /// PUBLISH — 发布消息到频道
    pub async fn publish(&self, channel: &str, message: &str) -> crate::Result<i64> {
        let mut conn = self.conn().await?;
        conn.publish(self.key(channel), message)
            .await
            .map_err(|e| cmd_failed("PUBLISH", &e.to_string()))
    }
}
