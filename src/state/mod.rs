mod callbacks;

use serde::Deserialize;

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

#[derive(Clone)]
pub struct AppState {
    pub backend: Backend,
    #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
    pub db: database::Database,
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
    #[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
    pub socket: socket::SocketManager,
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
    pub valkey: redis::Valkey,
}

/// 内部用于反序列化的临时结构
#[derive(Deserialize)]
struct ConfigFile {
    backend: Backend,
    #[cfg(feature = "db-postgres")]
    postgres: database::PostgresConfig,
    #[cfg(feature = "db-sqlite")]
    sqlite: database::SqliteConfig,
    #[cfg(feature = "db-mysql")]
    mysql: database::MysqlConfig,
    #[cfg(feature = "github-oauth2")]
    github_oauth2: github_oauth2::GitHubOAuth2,
    #[cfg(feature = "oss")]
    oss: oss::Oss,
    #[cfg(feature = "cos")]
    cos: cos::Cos,
    #[cfg(feature = "snow")]
    snow: snow::SnowConfig,
    #[cfg(feature = "jwt")]
    token: token::Token,
    #[cfg(feature = "wx-virtual-pay")]
    wx_virtual_pay: wx_virtual_pay::WxVirtualPay,
    #[cfg(feature = "wx-sec-check")]
    wx_sec_check: wx_sec_check::WxSecCheck,
    #[cfg(any(
        feature = "wx-pay-h5",
        feature = "wx-pay-native",
        feature = "wx-pay-app",
        feature = "wx-pay-mini",
        feature = "wx-pay-js"
    ))]
    wx_pay: wx_pay::WxPay,
    #[cfg(any(
        feature = "wx-login-mini",
        feature = "wx-login-app",
        feature = "wx-login-web"
    ))]
    wxlogin: wxlogin::WxLogin,
    #[cfg(feature = "wx-official")]
    wx_official: wx_official::WxOfficial,
    #[cfg(feature = "email")]
    email: email::EmailConfig,
    #[cfg(feature = "sms-ali")]
    sms_ali: smsali::SmsAli,
    #[cfg(feature = "sms-tencent")]
    sms_tencent: smstencent::SmsTencent,
    #[cfg(feature = "amap")]
    amap: amap::Amap,
    #[cfg(feature = "tmap")]
    tmap: tmap::Tmap,
    #[cfg(feature = "ali-pay-web")]
    ali_pay: ali_pay::AliPay,
    #[cfg(feature = "trace")]
    tracing: trace::TracingConfig,
    #[cfg(feature = "rate-limit")]
    rate_limit: rate_limit::RateLimitModuleConfig,
    #[cfg(feature = "serve")]
    serve: serve::ServeConfig,
    #[cfg(feature = "push")]
    push: push::PushManager,
    #[cfg(feature = "redis")]
    redis: redis::RedisConfig,
    #[cfg(all(feature = "valkey", not(feature = "redis")))]
    valkey: redis::ValkeyConfig,
}

impl AppState {
    pub async fn new(path: String) -> crate::Result<Self> {
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            crate::Error::custom(50001, format!("Failed to read config '{}': {}", path, e))
        })?;

        #[allow(unused_mut)]
        #[cfg(any(
            feature = "wx-login-mini",
            feature = "wx-login-app",
            feature = "wx-login-web",
            feature = "oss",
            feature = "cos",
            feature = "github-oauth2",
            feature = "wx-virtual-pay",
            feature = "wx-sec-check",
            feature = "wx-pay-h5",
            feature = "wx-pay-native",
            feature = "wx-pay-app",
            feature = "wx-pay-mini",
            feature = "wx-pay-js",
            feature = "sms-ali",
            feature = "sms-tencent",
            feature = "amap",
            feature = "tmap",
            feature = "ali-pay-web",
            feature = "trace",
            feature = "rate-limit",
            feature = "serve",
            feature = "redis",
            feature = "valkey",
            feature = "wx-official",
            feature = "push",
            feature = "db-postgres",
            feature = "db-sqlite",
            feature = "db-mysql"
        ))]
        let mut config: ConfigFile = toml::from_str(&content).map_err(|e| {
            crate::Error::custom(50001, format!("Failed to parse config.toml: {}", e))
        })?;

        #[cfg(not(any(
            feature = "wx-login-mini",
            feature = "wx-login-app",
            feature = "wx-login-web",
            feature = "oss",
            feature = "cos",
            feature = "github-oauth2",
            feature = "wx-virtual-pay",
            feature = "wx-sec-check",
            feature = "wx-pay-h5",
            feature = "wx-pay-native",
            feature = "wx-pay-app",
            feature = "wx-pay-mini",
            feature = "wx-pay-js",
            feature = "sms-ali",
            feature = "sms-tencent",
            feature = "amap",
            feature = "tmap",
            feature = "ali-pay-web",
            feature = "trace",
            feature = "rate-limit",
            feature = "serve",
            feature = "redis",
            feature = "valkey",
            feature = "wx-official",
            feature = "push",
            feature = "db-postgres",
            feature = "db-sqlite",
            feature = "db-mysql"
        )))]
        let config: ConfigFile = toml::from_str(&content).map_err(|e| {
            crate::Error::custom(50001, format!("Failed to parse config.toml: {}", e))
        })?;

        // 初始化 reqwest clients
        #[cfg(any(
            feature = "wx-login-mini",
            feature = "wx-login-app",
            feature = "wx-login-web"
        ))]
        {
            config.wxlogin.client = reqwest::Client::new();
        }
        #[cfg(feature = "wx-official")]
        {
            config.wx_official.client = reqwest::Client::new();
        }
        #[cfg(feature = "oss")]
        {
            config.oss.client = reqwest::Client::new();
        }
        #[cfg(feature = "cos")]
        {
            config.cos.client = reqwest::Client::new();
        }
        #[cfg(feature = "github-oauth2")]
        {
            config.github_oauth2.client = reqwest::Client::new();
        }
        #[cfg(feature = "wx-virtual-pay")]
        {
            config.wx_virtual_pay.client = reqwest::Client::new();
        }
        #[cfg(feature = "wx-sec-check")]
        {
            config.wx_sec_check.client = reqwest::Client::new();
        }
        #[cfg(any(
            feature = "wx-pay-h5",
            feature = "wx-pay-native",
            feature = "wx-pay-app",
            feature = "wx-pay-mini",
            feature = "wx-pay-js"
        ))]
        {
            config.wx_pay.init()?;
        }
        #[cfg(feature = "sms-ali")]
        {
            config.sms_ali.client = reqwest::Client::new();
        }
        #[cfg(feature = "sms-tencent")]
        {
            config.sms_tencent.client = reqwest::Client::new();
        }
        #[cfg(feature = "amap")]
        {
            config.amap.client = reqwest::Client::new();
        }
        #[cfg(feature = "tmap")]
        {
            config.tmap.client = reqwest::Client::new();
        }
        #[cfg(feature = "ali-pay-web")]
        {
            config.ali_pay.init()?;
        }

        // 初始化 Redis/Valkey
        #[cfg(feature = "redis")]
        let redis_client = redis::Redis::connect(&config.redis).await?;

        #[cfg(all(feature = "valkey", not(feature = "redis")))]
        let valkey_client = redis::Redis::connect(&config.valkey).await?;

        // 初始化数据库连接池
        #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
        let db = database::Database::connect(
            #[cfg(feature = "db-postgres")]
            &config.postgres,
            #[cfg(feature = "db-sqlite")]
            &config.sqlite,
            #[cfg(feature = "db-mysql")]
            &config.mysql,
        )
        .await?;

        let state = AppState {
            backend: config.backend,
            #[cfg(any(feature = "db-postgres", feature = "db-sqlite", feature = "db-mysql"))]
            db,
            #[cfg(feature = "clock")]
            clock: clock::Clock::new(),
            #[cfg(feature = "nonce")]
            nonce: nonce::Nonce::new(),
            #[cfg(feature = "github-oauth2")]
            github_oauth2: config.github_oauth2,
            #[cfg(feature = "oss")]
            oss: config.oss,
            #[cfg(feature = "cos")]
            cos: config.cos,
            #[cfg(feature = "snow")]
            snow: snow::Snowflake::from_config(&config.snow),
            #[cfg(feature = "jwt")]
            token: config.token,
            #[cfg(feature = "wx-virtual-pay")]
            wx_virtual_pay: config.wx_virtual_pay,
            #[cfg(feature = "wx-sec-check")]
            wx_sec_check: config.wx_sec_check,
            #[cfg(any(
                feature = "wx-pay-h5",
                feature = "wx-pay-native",
                feature = "wx-pay-app",
                feature = "wx-pay-mini",
                feature = "wx-pay-js"
            ))]
            wx_pay: config.wx_pay,
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web"
            ))]
            wxlogin: config.wxlogin,
            #[cfg(feature = "wx-official")]
            wx_official: config.wx_official,
            #[cfg(feature = "push")]
            push: {
                let mut pm = config.push;
                pm.init();
                pm
            },
            #[cfg(feature = "email")]
            email: email::Email::from_config(&config.email)?,
            #[cfg(feature = "regex-util")]
            regex_util: regex::RegexUtil::new(),
            #[cfg(feature = "sms-ali")]
            sms_ali: config.sms_ali,
            #[cfg(feature = "sms-tencent")]
            sms_tencent: config.sms_tencent,
            #[cfg(feature = "amap")]
            amap: config.amap,
            #[cfg(feature = "tmap")]
            tmap: config.tmap,
            #[cfg(feature = "ali-pay-web")]
            ali_pay: config.ali_pay,
            #[cfg(feature = "trace")]
            tracing: trace::init_tracing(&config.tracing).await?,
            #[cfg(feature = "rbac")]
            rbac: None,
            #[cfg(feature = "memkv")]
            memkv: memkv::MemKV::new(),
            #[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
            socket: socket::SocketManager::new(),
            #[cfg(feature = "scheduler")]
            scheduler: scheduler::Scheduler::new(),
            #[cfg(feature = "rate-limit")]
            rate_limit_config: config.rate_limit,
            #[cfg(feature = "serve")]
            serve: Some(serve::Serve::from_config(&config.serve, "./static")),
            #[cfg(feature = "redis")]
            redis: redis_client,
            #[cfg(all(feature = "valkey", not(feature = "redis")))]
            valkey: valkey_client,
        };

        // 初始化 Scheduler 的 state 引用
        #[cfg(feature = "scheduler")]
        state.scheduler.init_state(state.clone()).await;

        Ok(state)
    }

    /// 链式配置 GitHubOAuth2
    #[cfg(feature = "github-oauth2")]
    pub fn with_github_oauth2(
        mut self,
        f: impl FnOnce(github_oauth2::GitHubOAuth2) -> github_oauth2::GitHubOAuth2,
    ) -> Self {
        self.github_oauth2 = f(self.github_oauth2);
        self
    }

    /// 链式配置 WxVirtualPay
    #[cfg(feature = "wx-virtual-pay")]
    pub fn with_wx_virtual_pay(
        mut self,
        f: impl FnOnce(wx_virtual_pay::WxVirtualPay) -> wx_virtual_pay::WxVirtualPay,
    ) -> Self {
        self.wx_virtual_pay = f(self.wx_virtual_pay);
        self
    }

    /// 链式配置 WxPay
    #[cfg(any(
        feature = "wx-pay-h5",
        feature = "wx-pay-native",
        feature = "wx-pay-app",
        feature = "wx-pay-mini",
        feature = "wx-pay-js"
    ))]
    pub fn with_wx_pay(mut self, f: impl FnOnce(wx_pay::WxPay) -> wx_pay::WxPay) -> Self {
        self.wx_pay = f(self.wx_pay);
        self
    }

    /// 链式配置 WxLogin
    #[cfg(any(
        feature = "wx-login-mini",
        feature = "wx-login-app",
        feature = "wx-login-web"
    ))]
    pub fn with_wxlogin(mut self, f: impl FnOnce(wxlogin::WxLogin) -> wxlogin::WxLogin) -> Self {
        self.wxlogin = f(self.wxlogin);
        self
    }

    /// 链式配置 WxOfficial
    #[cfg(feature = "wx-official")]
    pub fn with_wx_official(
        mut self,
        f: impl FnOnce(wx_official::WxOfficial) -> wx_official::WxOfficial,
    ) -> Self {
        self.wx_official = f(self.wx_official);
        self
    }

    /// 链式配置阿里云短信
    #[cfg(feature = "sms-ali")]
    pub fn with_sms_ali(mut self, f: impl FnOnce(smsali::SmsAli) -> smsali::SmsAli) -> Self {
        self.sms_ali = f(self.sms_ali);
        self
    }

    /// 链式配置腾讯云短信
    #[cfg(feature = "sms-tencent")]
    pub fn with_sms_tencent(
        mut self,
        f: impl FnOnce(smstencent::SmsTencent) -> smstencent::SmsTencent,
    ) -> Self {
        self.sms_tencent = f(self.sms_tencent);
        self
    }

    /// 链式配置支付宝
    #[cfg(feature = "ali-pay-web")]
    pub fn with_ali_pay(mut self, f: impl FnOnce(ali_pay::AliPay) -> ali_pay::AliPay) -> Self {
        self.ali_pay = f(self.ali_pay);
        self
    }

    /// 设置 RBAC 实例
    #[cfg(feature = "rbac")]
    pub fn set_rbac(mut self, rbac: rbac::Rbac) -> Self {
        self.rbac = Some(rbac);
        self
    }

    /// 设置 MemKV 实例
    #[cfg(feature = "memkv")]
    pub fn with_memkv(mut self, memkv: memkv::MemKV) -> Self {
        self.memkv = memkv;
        self
    }

    /// 链式注册定时任务
    #[cfg(feature = "scheduler")]
    pub fn with_scheduler(
        mut self,
        f: impl FnOnce(scheduler::Scheduler) -> scheduler::Scheduler,
    ) -> Self {
        self.scheduler = f(self.scheduler);
        self
    }

    /// 批量注册定时任务
    #[cfg(feature = "scheduler")]
    pub fn schedulers(
        mut self,
        fs: Vec<Box<dyn FnOnce(scheduler::Scheduler) -> scheduler::Scheduler>>,
    ) -> Self {
        for f in fs {
            self.scheduler = f(self.scheduler);
        }
        self
    }

    /// 获取 Socket 连接管理器引用
    #[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
    pub fn socket(&self) -> &socket::SocketManager {
        &self.socket
    }

    /// 注册自定义 TraceStore
    ///
    /// 用于替换默认的 SQLite 存储后端。传入实现了 `TraceStore` trait 的类型即可。
    ///
    /// # 示例
    /// ```ignore
    /// let my_store = MyCustomStore::new().await?;
    /// let state = AppState::new("config.toml".into()).await?
    ///     .set_trace_store(my_store);
    /// ```
    #[cfg(feature = "trace")]
    pub fn set_trace_store(mut self, store: impl trace::TraceStore) -> Self {
        self.tracing.store = std::sync::Arc::new(store);
        self
    }
}

#[derive(Clone, Deserialize)]
pub struct Backend {
    pub host: String,
    pub port: u16,
    #[cfg(feature = "afast-tls")]
    #[serde(default)]
    pub tls: Option<TlsBackend>,
}

/// TLS 后端配置
#[cfg(feature = "afast-tls")]
#[derive(Clone, Deserialize)]
pub struct TlsBackend {
    /// 监听端口（默认 443）
    #[serde(default = "default_tls_port")]
    pub port: u16,
    /// PEM 证书链文件路径
    pub cert_path: String,
    /// PEM 私钥文件路径
    pub key_path: String,
}

#[cfg(feature = "afast-tls")]
fn default_tls_port() -> u16 {
    443
}
