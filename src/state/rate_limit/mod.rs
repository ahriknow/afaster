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

impl Default for ConfigKey {
    fn default() -> Self {
        Self::Ip
    }
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

impl Default for ConfigAlgorithm {
    fn default() -> Self {
        Self::TokenBucket
    }
}

/// 单条限流策略配置
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
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

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            max_requests: 0,
            window_seconds: 0,
            key: default_key(),
            algorithm: default_algorithm(),
            header_name: None,
        }
    }
}

fn default_key() -> ConfigKey {
    ConfigKey::Ip
}

fn default_algorithm() -> ConfigAlgorithm {
    ConfigAlgorithm::TokenBucket
}

/// 限流模块全局配置（对应 config.toml 的 [rate_limit]）
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
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

impl Default for RateLimitModuleConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            default_policy: None,
            rejected_code: default_rejected_code(),
            rejected_message: default_rejected_message(),
            policies: Vec::new(),
        }
    }
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

impl RateLimitModuleConfig {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let map = table
            .get("rate_limit")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        let policy_values = match map.get("policies") {
            Some(value) => value.as_array().cloned().ok_or_else(|| {
                crate::Error::custom(50001, "[rate_limit].policies must be an array of tables")
            })?,
            None => Vec::new(),
        };
        let mut policy_indexes = std::collections::BTreeSet::new();
        for (name, _) in std::env::vars_os() {
            let name = name.to_string_lossy();
            if let Some(index) = name
                .strip_prefix("RATE_LIMIT_POLICIES_")
                .and_then(|suffix| suffix.split('_').next())
                .and_then(|index| index.parse::<usize>().ok())
            {
                policy_indexes.insert(index);
            }
        }
        let policy_count = policy_indexes.last().map_or(policy_values.len(), |index| {
            policy_values.len().max(index + 1)
        });
        let mut policies = Vec::with_capacity(policy_count);
        for index in 0..policy_count {
            let policy_map = policy_values
                .get(index)
                .and_then(toml::Value::as_table)
                .cloned()
                .unwrap_or_default();
            let prefix = format!("RATE_LIMIT_POLICIES_{index}");
            let id = match (policy_map.get("id"), std::env::var(format!("{prefix}_ID"))) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(
                            50001,
                            format!("[rate_limit].policies[{index}].id must be a string"),
                        )
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!(
                            "missing required configuration [rate_limit].policies[{index}].id or {prefix}_ID"
                        ),
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!("{prefix}_ID is not valid UTF-8"),
                    ));
                }
            };
            let max_requests = match (
                policy_map.get("max_requests"),
                std::env::var(format!("{prefix}_MAX_REQUESTS")),
            ) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid {prefix}_MAX_REQUESTS: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_integer()
                    .ok_or_else(|| {
                        crate::Error::custom(
                            50001,
                            format!(
                                "[rate_limit].policies[{index}].max_requests must be an integer"
                            ),
                        )
                    })?
                    .try_into()
                    .map_err(|_| {
                        crate::Error::custom(
                            50001,
                            format!("[rate_limit].policies[{index}].max_requests is out of range"),
                        )
                    })?,
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!(
                            "missing required configuration [rate_limit].policies[{index}].max_requests or {prefix}_MAX_REQUESTS"
                        ),
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!("{prefix}_MAX_REQUESTS is not valid UTF-8"),
                    ));
                }
            };
            let window_seconds = match (
                policy_map.get("window_seconds"),
                std::env::var(format!("{prefix}_WINDOW_SECONDS")),
            ) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid {prefix}_WINDOW_SECONDS: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_integer()
                    .ok_or_else(|| {
                        crate::Error::custom(
                            50001,
                            format!(
                                "[rate_limit].policies[{index}].window_seconds must be an integer"
                            ),
                        )
                    })?
                    .try_into()
                    .map_err(|_| {
                        crate::Error::custom(
                            50001,
                            format!(
                                "[rate_limit].policies[{index}].window_seconds is out of range"
                            ),
                        )
                    })?,
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!(
                            "missing required configuration [rate_limit].policies[{index}].window_seconds or {prefix}_WINDOW_SECONDS"
                        ),
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!("{prefix}_WINDOW_SECONDS is not valid UTF-8"),
                    ));
                }
            };
            let key = match (
                policy_map.get("key"),
                std::env::var(format!("{prefix}_KEY")),
            ) {
                (_, Ok(value)) => match value.as_str() {
                    "ip" => ConfigKey::Ip,
                    "header" => ConfigKey::Header,
                    "connection" => ConfigKey::Connection,
                    "global" => ConfigKey::Global,
                    _ => {
                        return Err(crate::Error::custom(
                            50001,
                            format!("Invalid {prefix}_KEY: {value}"),
                        ));
                    }
                },
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.clone().try_into().map_err(|error| {
                        crate::Error::custom(
                            50001,
                            format!("[rate_limit].policies[{index}].key: {error}"),
                        )
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_key(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!("{prefix}_KEY is not valid UTF-8"),
                    ));
                }
            };
            let algorithm = match (
                policy_map.get("algorithm"),
                std::env::var(format!("{prefix}_ALGORITHM")),
            ) {
                (_, Ok(value)) => match value.as_str() {
                    "fixed_window" => ConfigAlgorithm::FixedWindow,
                    "sliding_window" => ConfigAlgorithm::SlidingWindow,
                    "token_bucket" => ConfigAlgorithm::TokenBucket,
                    _ => {
                        return Err(crate::Error::custom(
                            50001,
                            format!("Invalid {prefix}_ALGORITHM: {value}"),
                        ));
                    }
                },
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.clone().try_into().map_err(|error| {
                        crate::Error::custom(
                            50001,
                            format!("[rate_limit].policies[{index}].algorithm: {error}"),
                        )
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_algorithm(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!("{prefix}_ALGORITHM is not valid UTF-8"),
                    ));
                }
            };
            let header_name = match (
                policy_map.get("header_name"),
                std::env::var(format!("{prefix}_HEADER_NAME")),
            ) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(
                                50001,
                                format!(
                                    "[rate_limit].policies[{index}].header_name must be a string"
                                ),
                            )
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        format!("{prefix}_HEADER_NAME is not valid UTF-8"),
                    ));
                }
            };
            policies.push(PolicyConfig {
                id,
                max_requests,
                window_seconds,
                key,
                algorithm,
                header_name,
            });
        }
        Ok(Self {
            enabled: match (map.get("enabled"), std::env::var("RATE_LIMIT_ENABLED")) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(50001, format!("Invalid RATE_LIMIT_ENABLED: {error}"))
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.as_bool().ok_or_else(|| {
                        crate::Error::custom(50001, "[rate_limit].enabled must be a boolean")
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_enabled(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "RATE_LIMIT_ENABLED is not valid UTF-8",
                    ));
                }
            },
            default_policy: match (
                map.get("default_policy"),
                std::env::var("RATE_LIMIT_DEFAULT_POLICY"),
            ) {
                (_, Ok(value)) => Some(value),
                (Some(value), Err(std::env::VarError::NotPresent)) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            crate::Error::custom(
                                50001,
                                "[rate_limit].default_policy must be a string",
                            )
                        })?
                        .to_owned(),
                ),
                (None, Err(std::env::VarError::NotPresent)) => None,
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "RATE_LIMIT_DEFAULT_POLICY is not valid UTF-8",
                    ));
                }
            },
            rejected_code: match (
                map.get("rejected_code"),
                std::env::var("RATE_LIMIT_REJECTED_CODE"),
            ) {
                (_, Ok(value)) => value.parse().map_err(|error| {
                    crate::Error::custom(
                        50001,
                        format!("Invalid RATE_LIMIT_REJECTED_CODE: {error}"),
                    )
                })?,
                (Some(value), Err(std::env::VarError::NotPresent)) => {
                    value.as_integer().ok_or_else(|| {
                        crate::Error::custom(50001, "[rate_limit].rejected_code must be an integer")
                    })?
                }
                (None, Err(std::env::VarError::NotPresent)) => default_rejected_code(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "RATE_LIMIT_REJECTED_CODE is not valid UTF-8",
                    ));
                }
            },
            rejected_message: match (
                map.get("rejected_message"),
                std::env::var("RATE_LIMIT_REJECTED_MESSAGE"),
            ) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(
                            50001,
                            "[rate_limit].rejected_message must be a string",
                        )
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => default_rejected_message(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "RATE_LIMIT_REJECTED_MESSAGE is not valid UTF-8",
                    ));
                }
            },
            policies,
        })
    }
}
