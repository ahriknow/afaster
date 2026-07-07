//! # AFaster
//!
//! AFaster 是一个基于 Rust (afast) 的高性能后端框架模板。
//!
//! ## 内置错误码
//!
//! 错误码为 5 位数字：第 1 位 `4`=用户错误 / `5`=内部错误，第 2~3 位模块编号，第 4~5 位序号。
//!
//! 完整错误码表见 [ERRORS.md](https://afaster.ahriknow.help/errors.html)。

// ═══════════════════════════════════════════════════════════════
//  模块声明
// ═══════════════════════════════════════════════════════════════

mod authentication;
mod error;
mod handler;
#[cfg(feature = "log")]
mod logger;
mod state;

// ═══════════════════════════════════════════════════════════════
//  公共导出：核心
// ═══════════════════════════════════════════════════════════════

pub use afast::Error as AfastError;
#[cfg(feature = "afast-ordinary-http")]
pub use afast::{Html, HttpResult, Json, Text};
pub use authentication::AuthData;
pub use error::{Error, Result};
pub use state::AppState;

#[cfg(feature = "log")]
pub use tracing::{debug, error, info, trace, warn};

// ═══════════════════════════════════════════════════════════════
//  公共导出：微信生态
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "wx-official")]
pub use state::wx_official;
#[cfg(any(
    feature = "wx-pay-h5",
    feature = "wx-pay-native",
    feature = "wx-pay-app",
    feature = "wx-pay-mini",
    feature = "wx-pay-js"
))]
pub use state::wx_pay;
#[cfg(feature = "wx-sec-check")]
pub use state::wx_sec_check;
#[cfg(feature = "wx-virtual-pay")]
pub use state::wx_virtual_pay;

// ═══════════════════════════════════════════════════════════════
//  公共导出：阿里云
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "ali-pay-web")]
pub use state::ali_pay;
#[cfg(feature = "oss")]
pub use state::oss;
#[cfg(feature = "sms-ali")]
pub use state::smsali;

// ═══════════════════════════════════════════════════════════════
//  公共导出：腾讯云
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "cos")]
pub use state::cos;
#[cfg(feature = "sms-tencent")]
pub use state::smstencent;
#[cfg(feature = "tmap")]
pub use state::tmap;

// ═══════════════════════════════════════════════════════════════
//  公共导出：GitHub
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "github-oauth2")]
pub use state::github_oauth2;

// ═══════════════════════════════════════════════════════════════
//  公共导出：存储与数据
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "bloom")]
pub use state::bloom;
#[cfg(feature = "memkv")]
pub use state::memkv;
#[cfg(feature = "rbac")]
pub use state::rbac;
#[cfg(feature = "redis")]
pub use state::redis;
#[cfg(all(feature = "valkey", not(feature = "redis")))]
pub use state::redis;

// ═══════════════════════════════════════════════════════════════
//  公共导出：文件处理
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "excel")]
pub use state::excel;
#[cfg(feature = "file")]
pub use state::file;
#[cfg(feature = "image")]
pub use state::image;
#[cfg(feature = "pdf")]
pub use state::pdf;
#[cfg(feature = "serve")]
pub use state::serve;

// ═══════════════════════════════════════════════════════════════
//  公共导出：消息通知
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "email")]
pub use state::email;
#[cfg(feature = "push")]
pub use state::push;

// ═══════════════════════════════════════════════════════════════
//  公共导出：工具
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "acme")]
pub use state::acme;
#[cfg(feature = "amap")]
pub use state::amap;
#[cfg(feature = "argon2-hash")]
pub use state::argon2;
#[cfg(feature = "rate-limit")]
pub use state::rate_limit;
#[cfg(feature = "regex-util")]
pub use state::regex;
#[cfg(feature = "scheduler")]
pub use state::scheduler;
#[cfg(feature = "snow")]
pub use state::snow;
#[cfg(any(feature = "socket-binary", feature = "socket-ws", feature = "sse"))]
pub use state::socket;
#[cfg(feature = "afast-tls")]
pub use state::tls;
#[cfg(feature = "jwt")]
pub use state::token;
#[cfg(feature = "trace")]
pub use state::trace;

// ═══════════════════════════════════════════════════════════════
//  公共导出：构建器扩展 trait
// ═══════════════════════════════════════════════════════════════

#[cfg(feature = "acme")]
pub use state::acme::AFasterAcmeExt;
#[cfg(feature = "ali-pay-web")]
pub use state::ali_pay::AFasterAliPayExt;
#[cfg(feature = "bloom")]
pub use state::bloom::AFasterBloomExt;
#[cfg(feature = "github-oauth2")]
pub use state::github_oauth2::AFasterGitHubOAuth2Ext;
#[cfg(feature = "rbac")]
pub use state::rbac::AFasterRbacExt;
#[cfg(feature = "scheduler")]
pub use state::scheduler::AFasterSchedulerExt;
#[cfg(feature = "serve")]
pub use state::serve::AFasterServeExt;
#[cfg(feature = "sms-ali")]
pub use state::smsali::AFasterSmsAliExt;
#[cfg(feature = "sms-tencent")]
pub use state::smstencent::AFasterSmsTencentExt;
#[cfg(feature = "trace")]
pub use state::trace::AFasterTraceExt;
#[cfg(feature = "wx-official")]
pub use state::wx_official::AFasterWxOfficialExt;
#[cfg(any(
    feature = "wx-pay-h5",
    feature = "wx-pay-native",
    feature = "wx-pay-app",
    feature = "wx-pay-mini",
    feature = "wx-pay-js"
))]
pub use state::wx_pay::AFasterWxPayExt;
#[cfg(feature = "wx-virtual-pay")]
pub use state::wx_virtual_pay::AFasterWxVirtualPayExt;
#[cfg(any(
    feature = "wx-login-mini",
    feature = "wx-login-app",
    feature = "wx-login-web"
))]
pub use state::wxlogin::AFasterWxLoginExt;

// ═══════════════════════════════════════════════════════════════
//  AFaster 应用构建器
// ═══════════════════════════════════════════════════════════════

use afast::{AFast, service};
use handler::health;

/// AFaster 应用构建器
///
/// # 示例配置
#[doc = concat!("```toml\n", include_str!("../config.toml"), "\n```")]
pub struct AFaster {
    pub(crate) state: AppState,
    pub(crate) services: Vec<afast::Service>,
    #[cfg(any(feature = "afast-ts", feature = "afast-js", feature = "afast-kt"))]
    pub(crate) generate_targets: Vec<afast::GenerateTarget>,
    #[cfg(feature = "afast-doc")]
    pub(crate) doc_title: Option<String>,
    #[cfg(feature = "ext")]
    pub(crate) states: Vec<Box<dyn std::any::Any + Send + Sync>>,
    pub(crate) security_headers: Option<Vec<(&'static str, &'static str)>>,
}

// ── 构造与基础配置 ──────────────────────────────────────────

impl AFaster {
    /// 创建新的 AFaster 实例
    pub async fn new(path: String) -> crate::Result<Self> {
        Ok(Self {
            state: AppState::new(path).await?,
            services: Vec::new(),
            #[cfg(any(feature = "afast-ts", feature = "afast-js", feature = "afast-kt"))]
            generate_targets: Vec::new(),
            #[cfg(feature = "afast-doc")]
            doc_title: None,
            #[cfg(feature = "ext")]
            states: Vec::new(),
            security_headers: None,
        })
    }

    // 获取系统 AppState
    pub fn get_state(self) -> AppState {
        self.state
    }

    /// 设置任意类型的全局状态
    #[cfg(feature = "ext")]
    pub fn set_state(mut self, state: impl std::any::Any + Send + Sync) -> Self {
        self.states.push(Box::new(state));
        self
    }

    /// 注册服务
    pub fn service(mut self, svc: afast::Service) -> Self {
        self.services.push(svc);
        self
    }

    /// 批量注册服务
    pub fn services(mut self, svcs: Vec<afast::Service>) -> Self {
        self.services.extend(svcs);
        self
    }

    /// 添加代码生成目标
    #[cfg(any(feature = "afast-ts", feature = "afast-js", feature = "afast-kt"))]
    pub fn generate(mut self, target: afast::GenerateTarget) -> Self {
        self.generate_targets.push(target);
        self
    }

    /// 设置 API 文档标题
    #[cfg(feature = "afast-doc")]
    pub fn doc_title(mut self, title: impl Into<String>) -> Self {
        self.doc_title = Some(title.into());
        self
    }

    /// 覆盖默认的安全响应头
    ///
    /// 默认值为 `x-content-type-options: nosniff`, `x-frame-options: DENY`,
    /// `content-security-policy: default-src 'self'`。
    /// 调用此方法将完全替换默认列表。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// AFaster::new("config.toml".into()).await?
    ///     .with_security_headers(vec![
    ///         ("x-content-type-options", "nosniff"),
    ///         ("x-frame-options", "DENY"),
    ///         ("content-security-policy", "default-src 'self'; style-src 'self' 'unsafe-inline'"),
    ///     ])
    ///     .run().await;
    /// ```
    pub fn with_security_headers(mut self, headers: Vec<(&'static str, &'static str)>) -> Self {
        self.security_headers = Some(headers);
        self
    }
}

// ── 运行 ────────────────────────────────────────────────────

impl AFaster {
    /// 运行应用
    pub async fn run(self) {
        #[cfg(feature = "log")]
        let g = logger::init_logger();

        let health_svc = service!("health", "Health Check API" => {
            h(health),
        });

        let state = self.state;

        // 初始化 Scheduler 的 state 引用
        #[cfg(feature = "scheduler")]
        state.scheduler.init_state(state.clone()).await;

        // 提取地址（state 即将被 move）
        #[allow(unused)]
        let addr = format!("{}:{}", state.backend.host, state.backend.port);

        // ACME: 检查缓存证书，或在后台申请
        #[cfg(feature = "acme")]
        let acme_certs = state.acme.init_certs(state.clone());

        // 泄漏路径为 &'static str
        let lp = leak_paths(&state);

        // 提取限流配置（state 即将被 move）
        #[cfg(feature = "rate-limit")]
        let rl_config = state.rate_limit_config.to_afast_config();

        // 校验链路追踪
        #[cfg(feature = "trace")]
        let tracing_svc_ref = state.tracing.clone();
        #[cfg(feature = "trace")]
        if state.tracing.store().is_empty() {
            panic!("链路追踪未配置存储后端，请设置 [tracing].db_path 或调用 set_trace_store()");
        }

        let mut app = AFast::new().state(state.clone()).service(health_svc);

        // 安全响应头
        if let Some(headers) = self.security_headers {
            app = app.security_headers(headers);
        }

        // 链路追踪 hook + 服务
        #[cfg(feature = "trace")]
        {
            app = app.hook(state::trace::create_hook(&tracing_svc_ref));
            app = app.service(service!("_tracing", "链路追踪" => {
                h(state::trace::handler::report),
                h(state::trace::handler::report_batch),
                h(state::trace::handler::get_trace),
                h(state::trace::handler::get_children),
                h(state::trace::handler::list_traces),
                h(state::trace::handler::stats),
                get(lp.tracing_url, state::trace::page::tracing_page),
                sse(lp.tracing_event, state::trace::handler::trace_events),
            }));
        }

        // 布隆过滤器 hook
        #[cfg(feature = "bloom")]
        {
            app = app.hook(state::bloom::create_hook(&state.bloom));
        }

        // 扩展状态
        #[cfg(feature = "ext")]
        for ext_state in self.states {
            app = app.state(ext_state);
        }

        // ACME HTTP-01 挑战路由
        #[cfg(feature = "acme")]
        {
            app = app.service(service!("_acme", "ACME HTTP-01 Challenge" => {
                get(".well-known/acme-challenge/*", state::acme::challenge_handler),
            }));
        }

        // 回调路由
        app = register_callbacks(app, &lp);

        // 用户服务
        for svc in self.services {
            app = app.service(svc);
        }

        // 静态文件服务（catch-all, 应最后注册）
        #[cfg(feature = "serve")]
        if !state.serve.is_empty() {
            app = app.service(service!("_serve", "Static File Server" => {
                get("*", state::serve::serve_handler),
            }));
        }

        // 限流
        #[cfg(feature = "rate-limit")]
        if let Some(rl_config) = rl_config {
            app = app.rate_limit(rl_config);
        }

        // 文档
        #[cfg(feature = "afast-doc")]
        {
            app = app.document(afast::DocConfig {
                title: Some(self.doc_title.unwrap_or_else(|| "AFaster API Docs".into())),
                output: None,
            });
        }

        // 代码生成
        #[cfg(any(feature = "afast-ts", feature = "afast-js", feature = "afast-kt"))]
        {
            app = app.generate(self.generate_targets);
        }

        // 启动
        #[cfg(feature = "afast-http")]
        {
            app = app.http(&addr);
        }

        // 取出 TLS 重载通道 Receiver
        #[cfg(feature = "afast-tls")]
        let mut tls_reload_rx = state.tls.take_receiver().await;

        // 启动 HTTPS（ACME 证书）
        #[cfg(all(feature = "afast-tls", feature = "acme"))]
        if let Some((ref cert_path, ref key_path)) = acme_certs {
            let tls_addr = format!("{}:{}", state.backend.host, state.tls.config.port);
            app = app.https(
                &tls_addr,
                &cert_path.to_string_lossy(),
                &key_path.to_string_lossy(),
                tls_reload_rx.take(),
            );
        }

        // 手动 TLS（ACME 已启用但未获取证书时的回退）
        #[cfg(all(feature = "afast-tls", feature = "acme"))]
        if acme_certs.is_none() {
            let tls_config = &state.tls.config;
            let tls_addr = format!("{}:{}", state.backend.host, tls_config.port);
            app = app.https(
                &tls_addr,
                &tls_config.cert_path,
                &tls_config.key_path,
                tls_reload_rx.take(),
            );
        }

        // 手动 TLS（仅在 ACME 未启用时使用）
        #[cfg(all(feature = "afast-tls", not(feature = "acme")))]
        {
            let tls_config = &state.tls.config;
            let tls_addr = format!("{}:{}", state.backend.host, tls_config.port);
            app = app.https(
                &tls_addr,
                &tls_config.cert_path,
                &tls_config.key_path,
                tls_reload_rx.take(),
            );
        }

        #[cfg(feature = "afast-ws")]
        {
            app = app.ws(&addr);
        }

        app.run().await.unwrap();

        #[cfg(feature = "log")]
        {
            _ = g;
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  内部辅助
// ═══════════════════════════════════════════════════════════════

/// 泄漏的路径字符串
struct LeakedPaths {
    #[cfg(feature = "trace")]
    tracing_url: &'static str,
    #[cfg(feature = "trace")]
    tracing_event: &'static str,
    // 微信生态
    #[cfg(any(
        feature = "wx-pay-h5",
        feature = "wx-pay-native",
        feature = "wx-pay-app",
        feature = "wx-pay-mini",
        feature = "wx-pay-js"
    ))]
    wx_pay_callback: &'static str,
    #[cfg(feature = "wx-virtual-pay")]
    wx_virtual_pay_callback: &'static str,
    #[cfg(feature = "wx-login-web")]
    wx_web_login_callback: &'static str,
    #[cfg(feature = "wx-official")]
    wx_mp_login_callback: &'static str,
    #[cfg(feature = "wx-sec-check")]
    wx_sec_check_callback: &'static str,
    // 阿里云
    #[cfg(feature = "ali-pay-web")]
    ali_pay_callback: &'static str,
    #[cfg(feature = "sms-ali")]
    sms_ali_callback: &'static str,
    // 腾讯云
    #[cfg(feature = "sms-tencent")]
    sms_tencent_callback: &'static str,
    // GitHub
    #[cfg(feature = "github-oauth2")]
    github_callback: &'static str,
}

fn leak_paths(#[allow(unused)] state: &AppState) -> LeakedPaths {
    LeakedPaths {
        #[cfg(feature = "trace")]
        tracing_url: Box::leak(state.tracing.config().url.clone().into_boxed_str()),
        #[cfg(feature = "trace")]
        tracing_event: Box::leak(state.tracing.config().event.clone().into_boxed_str()),
        // 微信生态
        #[cfg(any(
            feature = "wx-pay-h5",
            feature = "wx-pay-native",
            feature = "wx-pay-app",
            feature = "wx-pay-mini",
            feature = "wx-pay-js"
        ))]
        wx_pay_callback: Box::leak(state.wx_pay.callback_path.clone().into_boxed_str()),
        #[cfg(feature = "wx-virtual-pay")]
        wx_virtual_pay_callback: Box::leak(
            state.wx_virtual_pay.callback_path.clone().into_boxed_str(),
        ),
        #[cfg(feature = "wx-login-web")]
        wx_web_login_callback: Box::leak(state.wxlogin.callback_path.clone().into_boxed_str()),
        #[cfg(feature = "wx-official")]
        wx_mp_login_callback: Box::leak(state.wx_official.callback_path.clone().into_boxed_str()),
        #[cfg(feature = "wx-sec-check")]
        wx_sec_check_callback: Box::leak(state.wx_sec_check.callback_path.clone().into_boxed_str()),
        // 阿里云
        #[cfg(feature = "ali-pay-web")]
        ali_pay_callback: Box::leak(state.ali_pay.callback_path.clone().into_boxed_str()),
        #[cfg(feature = "sms-ali")]
        sms_ali_callback: Box::leak(state.sms_ali.callback_path.clone().into_boxed_str()),
        // 腾讯云
        #[cfg(feature = "sms-tencent")]
        sms_tencent_callback: Box::leak(state.sms_tencent.callback_path.clone().into_boxed_str()),
        // GitHub
        #[cfg(feature = "github-oauth2")]
        github_callback: Box::leak(state.github_oauth2.callback_path.clone().into_boxed_str()),
    }
}

fn register_callbacks(
    #[allow(unused_mut)] mut app: AFast,
    #[allow(unused)] lp: &LeakedPaths,
) -> AFast {
    // ── 微信生态回调 ──

    #[cfg(any(
        feature = "wx-pay-h5",
        feature = "wx-pay-native",
        feature = "wx-pay-app",
        feature = "wx-pay-mini",
        feature = "wx-pay-js"
    ))]
    {
        app = app.service(service!("", "微信支付回调" => {
            post(lp.wx_pay_callback, state::wx_pay::callback::wx_pay_notify_handler),
        }));
    }

    #[cfg(feature = "wx-virtual-pay")]
    {
        app = app.service(service!("", "微信虚拟支付回调" => {
            get(lp.wx_virtual_pay_callback, state::wx_virtual_pay::callback::verify),
            post(lp.wx_virtual_pay_callback, state::wx_virtual_pay::callback::dispatch),
        }));
    }

    #[cfg(feature = "wx-login-web")]
    {
        app = app.service(service!("", "微信网页扫码登录回调" => {
            get(lp.wx_web_login_callback, state::wxlogin::wx_web_login_callback_handler),
        }));
    }

    #[cfg(feature = "wx-official")]
    {
        app = app.service(service!("", "微信公众号网页授权回调" => {
            get(lp.wx_mp_login_callback, state::wx_official::wx_mp_login_callback_handler),
        }));
    }

    #[cfg(feature = "wx-sec-check")]
    {
        app = app.service(service!("", "微信内容安全回调" => {
            get(lp.wx_sec_check_callback, state::wx_sec_check::callback::verify),
            post(lp.wx_sec_check_callback, state::wx_sec_check::callback::dispatch),
        }));
    }

    // ── 阿里云回调 ──

    #[cfg(feature = "ali-pay-web")]
    {
        app = app.service(service!("", "支付宝回调" => {
            post(lp.ali_pay_callback, state::ali_pay::notify_handler),
        }));
    }

    #[cfg(feature = "sms-ali")]
    {
        app = app.service(service!("", "阿里云短信回执" => {
            post(lp.sms_ali_callback, state::smsali::sms_report_callback_handler),
        }));
    }

    // ── 腾讯云回调 ──

    #[cfg(feature = "sms-tencent")]
    {
        app = app.service(service!("", "腾讯云短信回执" => {
            post(lp.sms_tencent_callback, state::smstencent::sms_report_callback_handler),
        }));
    }

    // ── GitHub 回调 ──

    #[cfg(feature = "github-oauth2")]
    {
        app = app.service(service!("", "GitHub OAuth2" => {
            get(lp.github_callback, state::github_oauth2::github_oauth2_callback_handler),
        }));
    }

    app
}

// ═══════════════════════════════════════════════════════════════

pub const CONFIG_TEMPLATE: &str = include_str!("../config.toml");
