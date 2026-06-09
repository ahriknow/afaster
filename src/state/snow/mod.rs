use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[derive(Clone)]
pub struct Snowflake {
    epoch: i64,
    worker_id: i64,
    datacenter_id: i64,
    sequence: Arc<AtomicI64>,
    last_timestamp: Arc<AtomicI64>,
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
            sequence: Arc::new(AtomicI64::new(0)),
            last_timestamp: Arc::new(AtomicI64::new(-1)),
        }
    }

    pub fn next_id(&self) -> i64 {
        let mut timestamp = Self::current_time_millis();

        let last = self.last_timestamp.load(Ordering::SeqCst);

        if timestamp < last {
            panic!("Clock moved backwards");
        }

        let sequence = if timestamp == last {
            let seq = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
            if seq > 4095 {
                timestamp = self.wait_next_millis(last);
                self.sequence.store(0, Ordering::SeqCst);
                0
            } else {
                seq
            }
        } else {
            self.sequence.store(0, Ordering::SeqCst);
            0
        };

        self.last_timestamp.store(timestamp, Ordering::SeqCst);

        ((timestamp - self.epoch) << 22)
            | (self.datacenter_id << 17)
            | (self.worker_id << 12)
            | sequence
    }

    pub fn next_id_str(&self) -> String {
        self.next_id().to_string()
    }

    pub fn next_id_prefix(&self, prefix: &str) -> String {
        format!("{}{}", prefix, self.next_id())
    }

    fn wait_next_millis(&self, last: i64) -> i64 {
        let mut ts = Self::current_time_millis();
        while ts <= last {
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
