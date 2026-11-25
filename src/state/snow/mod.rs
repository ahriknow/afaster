use serde::Deserialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

fn default_epoch() -> i64 {
    0 // 0 表示使用当前时间
}

fn default_worker_id() -> i64 {
    1
}

fn default_datacenter_id() -> i64 {
    1
}

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct SnowConfig {
    /// 自定义起始时间（毫秒时间戳），0 表示使用当前时间
    #[serde(default = "default_epoch")]
    pub epoch: i64,
    /// 机器 ID (0~31)
    #[serde(default = "default_worker_id")]
    pub worker_id: i64,
    /// 数据中心 ID (0~31)
    #[serde(default = "default_datacenter_id")]
    pub datacenter_id: i64,
}

impl Default for SnowConfig {
    fn default() -> Self {
        Self {
            epoch: default_epoch(),
            worker_id: default_worker_id(),
            datacenter_id: default_datacenter_id(),
        }
    }
}

#[derive(Clone)]
pub struct Snowflake {
    epoch: i64,
    worker_id: i64,
    datacenter_id: i64,
    inner: Arc<Mutex<SnowflakeInner>>,
}

struct SnowflakeInner {
    sequence: i64,
    last_timestamp: i64,
}

impl Snowflake {
    /// 从配置创建 Snowflake 实例
    pub fn from_config(config: &SnowConfig) -> Self {
        assert!(
            config.worker_id >= 0 && config.worker_id < 32,
            "worker_id must be 0~31"
        );
        assert!(
            config.datacenter_id >= 0 && config.datacenter_id < 32,
            "datacenter_id must be 0~31"
        );

        let epoch = if config.epoch == 0 {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time error")
                .as_millis() as i64
        } else {
            config.epoch
        };

        Self {
            epoch,
            worker_id: config.worker_id,
            datacenter_id: config.datacenter_id,
            inner: Arc::new(Mutex::new(SnowflakeInner {
                sequence: 0,
                last_timestamp: -1,
            })),
        }
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let map = table
            .get("snow")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        let config = SnowConfig {
            epoch: match (map.get("epoch"), std::env::var("SNOW_EPOCH")) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid SNOW_EPOCH: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.as_integer().ok_or_else(|| {
                        crate::Error::custom(50001, "[snow].epoch must be an integer")
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_epoch(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(50001, "SNOW_EPOCH is not valid UTF-8"));
                }
            },
            worker_id: match (map.get("worker_id"), std::env::var("SNOW_WORKER_ID")) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid SNOW_WORKER_ID: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.as_integer().ok_or_else(|| {
                        crate::Error::custom(50001, "[snow].worker_id must be an integer")
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_worker_id(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "SNOW_WORKER_ID is not valid UTF-8",
                    ));
                }
            },
            datacenter_id: match (
                map.get("datacenter_id"),
                std::env::var("SNOW_DATACENTER_ID"),
            ) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid SNOW_DATACENTER_ID: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.as_integer().ok_or_else(|| {
                        crate::Error::custom(50001, "[snow].datacenter_id must be an integer")
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_datacenter_id(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "SNOW_DATACENTER_ID is not valid UTF-8",
                    ));
                }
            },
        };
        if !(0..32).contains(&config.worker_id) || !(0..32).contains(&config.datacenter_id) {
            return Err(crate::Error::custom(
                50001,
                "snow worker_id and datacenter_id must be in 0..32",
            ));
        }
        Ok(Self::from_config(&config))
    }

    pub async fn next_id(&self) -> i64 {
        let mut inner = self.inner.lock().await;
        let mut timestamp = Self::current_time_millis();

        // 时钟回拨时等待追上，而非 panic（NTP 调整、VM 迁移等场景可能发生）
        if timestamp < inner.last_timestamp {
            #[cfg(feature = "log")]
            tracing::warn!(
                "Snowflake: clock moved backwards by {}ms, waiting...",
                inner.last_timestamp - timestamp
            );
            timestamp = Self::wait_next_millis(inner.last_timestamp);
        }

        let sequence = if timestamp == inner.last_timestamp {
            inner.sequence += 1;
            if inner.sequence > 4095 {
                timestamp = Self::wait_next_millis(inner.last_timestamp);
                inner.sequence = 0;
                0
            } else {
                inner.sequence
            }
        } else {
            inner.sequence = 0;
            0
        };

        inner.last_timestamp = timestamp;

        ((timestamp - self.epoch) << 22)
            | (self.datacenter_id << 17)
            | (self.worker_id << 12)
            | sequence
    }

    pub async fn next_id_str(&self) -> String {
        self.next_id().await.to_string()
    }

    pub async fn next_id_prefix(&self, prefix: &str) -> String {
        format!("{}{}", prefix, self.next_id().await)
    }

    fn wait_next_millis(last: i64) -> i64 {
        let mut ts = Self::current_time_millis();
        while ts <= last {
            std::thread::yield_now();
            ts = Self::current_time_millis();
        }
        ts
    }

    fn current_time_millis() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time error")
            .as_millis() as i64
    }
}
