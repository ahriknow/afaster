mod callbacks;

use serde::Deserialize;

/// 从 toml::Table 中提取并反序列化指定 section
fn extract<T: serde::de::DeserializeOwned>(table: &toml::Table, key: &str) -> crate::Result<T> {
    table
        .get(key)
        .ok_or_else(|| crate::Error::custom(50001, format!("missing [{}]", key)))?
        .clone()
        .try_into()
        .map_err(|e| crate::Error::custom(50001, format!("[{}] {}", key, e)))
}

#[cfg(feature = "clock")]
pub mod clock;

#[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
pub mod database;

#[cfg(feature = "nonce")]
pub mod nonce;

#[cfg(feature = "github-oauth2")]
pub mod github_oauth2;

#[cfg(feature = "oss")]
pub mod oss;

#[cfg(feature = "cos")]
pub mod cos;

#[cfg(feature = "snow")]
pub mod snow;

#[cfg(feature = "jwt")]
pub mod token;

#[cfg(feature = "wx-virtual-pay")]
pub mod wx_virtual_pay;

#[cfg(feature = "wx-sec-check")]
pub mod wx_sec_check;

#[cfg(any(
    feature = "wx-pay-h5",
    feature = "wx-pay-native",
    feature = "wx-pay-app",
    feature = "wx-pay-mini",
    feature = "wx-pay-js"
))]
pub mod wx_pay;

#[cfg(feature = "email")]
pub mod email;

#[cfg(feature = "regex-util")]
pub mod regex;

#[cfg(feature = "sms-ali")]
pub mod smsali;

#[cfg(feature = "sms-tencent")]
pub mod smstencent;

#[cfg(feature = "amap")]
pub mod amap;

#[cfg(feature = "tmap")]
pub mod tmap;

#[cfg(feature = "ali-pay-web")]
pub mod ali_pay;

#[cfg(feature = "trace")]
pub mod trace;

#[cfg(feature = "rbac")]
pub mod rbac;

#[cfg(feature = "memkv")]
pub mod memkv;

#[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
pub mod socket;

#[cfg(feature = "acme")]
pub mod acme;

#[cfg(feature = "afast-tls")]
pub mod tls;

#[cfg(feature = "push")]
pub mod push;

#[cfg(feature = "scheduler")]
pub mod scheduler;

#[cfg(feature = "rate-limit")]
pub mod rate_limit;

#[cfg(feature = "serve")]
pub mod serve;

#[cfg(feature = "excel")]
pub mod excel;

#[cfg(feature = "pdf")]
pub mod pdf;

#[cfg(feature = "image")]
pub mod image;

#[cfg(feature = "file")]
pub mod file;

#[cfg(any(feature = "redis", feature = "valkey"))]
pub mod redis;

#[cfg(any(
    feature = "wx-login-mini",
    feature = "wx-login-app",
    feature = "wx-login-web"
))]
pub mod wxlogin;

#[cfg(feature = "wx-official")]
pub mod wx_official;

#[cfg(feature = "argon2-hash")]
pub mod argon2;

#[cfg(feature = "bloom")]
pub mod bloom;

#[derive(Clone)]
pub struct AppState {
    pub backend: Backend,
    #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
    pub db: database::Database,
    #[cfg(feature = "argon2-hash")]
    pub argon2: argon2::Argon2Hasher,
    #[cfg(feature = "clock")]
    pub clock: clock::Clock,
    #[cfg(feature = "nonce")]
    pub nonce: nonce::Nonce,
    #[cfg(feature = "github-oauth2")]
    pub github_oauth2: github_oauth2::GitHubOAuth2,
    #[cfg(feature = "oss")]
    pub oss: oss::Oss,
    #[cfg(feature = "cos")]
    pub cos: cos::Cos,
    #[cfg(feature = "snow")]
    pub snow: snow::Snowflake,
    #[cfg(feature = "jwt")]
    pub token: token::Token,
    #[cfg(feature = "wx-virtual-pay")]
    pub wx_virtual_pay: wx_virtual_pay::WxVirtualPay,
    #[cfg(feature = "wx-sec-check")]
    pub wx_sec_check: wx_sec_check::WxSecCheck,
    #[cfg(any(
        feature = "wx-pay-h5",
        feature = "wx-pay-native",
        feature = "wx-pay-app",
        feature = "wx-pay-mini",
        feature = "wx-pay-js"
    ))]
    pub wx_pay: wx_pay::WxPay,
    #[cfg(any(
        feature = "wx-login-mini",
        feature = "wx-login-app",
        feature = "wx-login-web"
    ))]
    pub wxlogin: wxlogin::WxLogin,
    #[cfg(feature = "wx-official")]
    pub wx_official: wx_official::WxOfficial,
    #[cfg(feature = "email")]
    pub email: email::Email,
    #[cfg(feature = "regex-util")]
    pub regex_util: regex::RegexUtil,
    #[cfg(feature = "sms-ali")]
    pub sms_ali: smsali::SmsAli,
    #[cfg(feature = "sms-tencent")]
    pub sms_tencent: smstencent::SmsTencent,
    #[cfg(feature = "amap")]
    pub amap: amap::Amap,
    #[cfg(feature = "tmap")]
    pub tmap: tmap::Tmap,
    #[cfg(feature = "ali-pay-web")]
    pub ali_pay: ali_pay::AliPay,
    #[cfg(feature = "trace")]
    pub tracing: trace::TracingService,
    #[cfg(feature = "rbac")]
    pub rbac: Option<rbac::Rbac>,
    #[cfg(feature = "memkv")]
    pub memkv: memkv::MemKV,
    #[cfg(feature = "bloom")]
    pub bloom: bloom::BloomFilter,
    #[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
    pub socket: socket::SocketManager,
    #[cfg(feature = "acme")]
    pub acme: acme::AcmeState,
    #[cfg(feature = "afast-tls")]
    pub tls: tls::Tls,
    #[cfg(feature = "push")]
    pub push: push::PushManager,
    #[cfg(feature = "scheduler")]
    pub scheduler: scheduler::Scheduler,
    #[cfg(feature = "rate-limit")]
    pub rate_limit_config: rate_limit::RateLimitModuleConfig,
    #[cfg(feature = "serve")]
    pub serve: Option<serve::Serve>,
    #[cfg(feature = "redis")]
    pub redis: redis::Redis,
    #[cfg(all(feature = "valkey", not(feature = "redis")))]
    pub valkey: redis::Redis,
}

impl AppState {
    pub async fn new(path: String) -> crate::Result<Self> {
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            crate::Error::custom(50001, format!("Failed to read config '{}': {}", path, e))
        })?;
        let table: toml::Table = toml::from_str(&content).map_err(|e| {
            crate::Error::custom(50001, format!("Failed to parse config.toml: {}", e))
        })?;

        let backend = Backend::from_table(&table)?;

        let state = AppState {
            backend: backend.clone(),
            #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
            db: database::Database::from_table(&table).await?,
            #[cfg(feature = "argon2-hash")]
            argon2: argon2::Argon2Hasher::new(),
            #[cfg(feature = "clock")]
            clock: clock::Clock::new(),
            #[cfg(feature = "nonce")]
            nonce: nonce::Nonce::new(),
            #[cfg(feature = "github-oauth2")]
            github_oauth2: github_oauth2::GitHubOAuth2::from_table(&table)?,
            #[cfg(feature = "oss")]
            oss: oss::Oss::from_table(&table)?,
            #[cfg(feature = "cos")]
            cos: cos::Cos::from_table(&table)?,
            #[cfg(feature = "snow")]
            snow: snow::Snowflake::from_table(&table)?,
            #[cfg(feature = "jwt")]
            token: token::Token::from_table(&table)?,
            #[cfg(feature = "wx-virtual-pay")]
            wx_virtual_pay: wx_virtual_pay::WxVirtualPay::from_table(&table)?,
            #[cfg(feature = "wx-sec-check")]
            wx_sec_check: wx_sec_check::WxSecCheck::from_table(&table)?,
            #[cfg(any(
                feature = "wx-pay-h5",
                feature = "wx-pay-native",
                feature = "wx-pay-app",
                feature = "wx-pay-mini",
                feature = "wx-pay-js"
            ))]
            wx_pay: wx_pay::WxPay::from_table(&table)?,
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web"
            ))]
            wxlogin: wxlogin::WxLogin::from_table(&table)?,
            #[cfg(feature = "wx-official")]
            wx_official: wx_official::WxOfficial::from_table(&table)?,
            #[cfg(feature = "push")]
            push: push::PushManager::from_table(&table)?,
            #[cfg(feature = "email")]
            email: email::Email::from_table(&table)?,
            #[cfg(feature = "regex-util")]
            regex_util: regex::RegexUtil::new(),
            #[cfg(feature = "sms-ali")]
            sms_ali: smsali::SmsAli::from_table(&table)?,
            #[cfg(feature = "sms-tencent")]
            sms_tencent: smstencent::SmsTencent::from_table(&table)?,
            #[cfg(feature = "amap")]
            amap: amap::Amap::from_table(&table)?,
            #[cfg(feature = "tmap")]
            tmap: tmap::Tmap::from_table(&table)?,
            #[cfg(feature = "ali-pay-web")]
            ali_pay: ali_pay::AliPay::from_table(&table)?,
            #[cfg(feature = "trace")]
            tracing: trace::TracingService::from_table(&table).await?,
            #[cfg(feature = "rbac")]
            rbac: None,
            #[cfg(feature = "memkv")]
            memkv: memkv::MemKV::new(),
            #[cfg(feature = "bloom")]
            bloom: bloom::BloomFilter::from_table(&table)?,
            #[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
            socket: socket::SocketManager::new(),
            #[cfg(feature = "acme")]
            acme: acme::AcmeState::from_table(&table, backend.port)?,
            #[cfg(feature = "afast-tls")]
            tls: tls::Tls::from_table(&table)?,
            #[cfg(feature = "scheduler")]
            scheduler: scheduler::Scheduler::new(),
            #[cfg(feature = "rate-limit")]
            rate_limit_config: rate_limit::RateLimitModuleConfig::from_table(&table)?,
            #[cfg(feature = "serve")]
            serve: Some(serve::Serve::from_table(&table)?),
            #[cfg(feature = "redis")]
            redis: redis::Redis::from_table(&table).await?,
            #[cfg(all(feature = "valkey", not(feature = "redis")))]
            valkey: redis::Redis::from_table(&table).await?,
        };

        Ok(state)
    }
}

#[derive(Clone, Deserialize)]
pub struct Backend {
    pub host: String,
    pub port: u16,
}

impl Backend {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        extract(table, "backend")
    }
}
