use serde::Deserialize;

// ═══════════════════════════════════════════════════════════════
//  限流配置模块
//  基于 afast 框架内置限流方案，从 config.toml 读取配置
// ═══════════════════════════════════════════════════════════════

// 重新导出 afast 限流类型，方便用户使用
pub use afast::rate_limit::{
    Algorithm, InMemoryStore, RateLimitConfig, RateLimitKey, RateLimitPolicy,
};

/// 限流 key 提取方式（配置文件用）
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigKey {
    /// 按客户端 IP 限流
    Ip,
    /// 按 HTTP Header 值限流
    Header,
    /// 按连接限流（WS/TCP 消息频率）
    Connection,
    /// 全局共享计数器
    Global,
}

/// 限流算法（配置文件用）
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfigAlgorithm {
    /// 固定窗口：在窗口边界重置
    FixedWindow,
    /// 滑动窗口：平滑滑动，避免边界突发
    SlidingWindow,
    /// 令牌桶：允许短时突发，以恒定速率补充令牌
    TokenBucket,
}

/// 单条限流策略配置
#[derive(Debug, Clone, Deserialize)]
pub struct PolicyConfig {
    /// 策略唯一标识（handler 通过此 ID 引用）
    pub id: String,
    /// 时间窗口内允许的最大请求数
    pub max_requests: u64,
    /// 时间窗口大小（秒）
    pub window_seconds: u64,
    /// key 提取方式，默认按 IP
    #[serde(default = "default_key")]
    pub key: ConfigKey,
    /// 限流算法，默认令牌桶
    #[serde(default = "default_algorithm")]
    pub algorithm: ConfigAlgorithm,
    /// Header 限流时的 Header 名称（key = "header" 时必填）
    #[serde(default)]
    pub header_name: Option<String>,
}

fn default_key() -> ConfigKey {
    ConfigKey::Ip
}

fn default_algorithm() -> ConfigAlgorithm {
    ConfigAlgorithm::TokenBucket
}

/// 限流模块全局配置（对应 config.toml 的 [rate_limit]）
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitModuleConfig {
    /// 是否启用限流
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// 默认策略 ID（未在 handler 上指定时使用）
    #[serde(default)]
    pub default_policy: Option<String>,
    /// 被拒绝时的错误码
    #[serde(default = "default_rejected_code")]
    pub rejected_code: i64,
    /// 被拒绝时的错误消息
    #[serde(default = "default_rejected_message")]
    pub rejected_message: String,
    /// 限流策略列表
    #[serde(default)]
    pub policies: Vec<PolicyConfig>,
}

fn default_enabled() -> bool {
    true
}

fn default_rejected_code() -> i64 {
    42001
}

fn default_rejected_message() -> String {
    "Too many requests".to_string()
}

impl RateLimitModuleConfig {
    /// 将配置转换为 afast 的 RateLimitConfig
    pub fn to_afast_config(&self) -> Option<RateLimitConfig> {
        if !self.enabled || self.policies.is_empty() {
            return None;
        }

        let mut config = RateLimitConfig::new()
            .rejected_code(self.rejected_code)
            .rejected_message(&self.rejected_message);

        if let Some(ref default_id) = self.default_policy {
            config = config.default_policy(default_id);
        }

        for policy in &self.policies {
            let key = match policy.key {
                ConfigKey::Ip => RateLimitKey::Ip,
                ConfigKey::Header => {
                    let name = policy.header_name.as_deref().unwrap_or("Authorization");
                    let static_name: &'static str = Box::leak(name.to_string().into_boxed_str());
                    RateLimitKey::Header(static_name)
                }
                ConfigKey::Connection => RateLimitKey::Connection,
                ConfigKey::Global => RateLimitKey::Global,
            };

            let algorithm = match policy.algorithm {
                ConfigAlgorithm::FixedWindow => Algorithm::FixedWindow,
                ConfigAlgorithm::SlidingWindow => Algorithm::SlidingWindow,
                ConfigAlgorithm::TokenBucket => Algorithm::TokenBucket,
            };

            config = config.policy(RateLimitPolicy {
                id: policy.id.clone(),
                max_requests: policy.max_requests,
                window_secs: policy.window_seconds,
                key,
                algorithm,
            });
        }

        Some(config)
    }
}
