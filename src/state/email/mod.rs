pub mod err;
use err::*;

use std::collections::HashMap;

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde::Deserialize;

#[cfg(feature = "log")]
use tracing::debug;

// ═══════════════════════════════════════════════════════════════
//  邮箱账户配置
// ═══════════════════════════════════════════════════════════════

/// 单个邮箱账户配置
#[derive(Clone, Deserialize)]
pub struct EmailAccountConfig {
    /// 唯一标识（不可重复）
    pub id: String,
    /// SMTP 服务器地址
    pub host: String,
    /// 端口（465=SSL, 587=STARTTLS）
    pub port: u16,
    /// 登录用户名
    pub user: String,
    /// 密码/授权码
    pub pass: String,
    /// 发件人地址
    pub from: String,
    /// 发件人显示名称
    pub name: String,
    /// 是否使用 SSL/TLS, 默认 true
    #[serde(default = "default_secure")]
    pub secure: bool,
    /// 是否使用 STARTTLS（587 端口）, 默认 false（隐式 SSL/465 端口）
    #[serde(default)]
    pub starttls: bool,
}

fn default_secure() -> bool {
    true
}

/// 邮件模块配置
#[derive(Clone, Deserialize)]
pub struct EmailConfig {
    /// 默认邮箱 ID
    pub default: String,
    /// 邮箱账户列表
    pub accounts: Vec<EmailAccountConfig>,
}

// ═══════════════════════════════════════════════════════════════
//  邮件服务
// ═══════════════════════════════════════════════════════════════

/// 邮件服务
#[derive(Clone)]
pub struct Email {
    accounts: HashMap<String, EmailAccountConfig>,
    default_id: String,
}

impl Email {
    /// 从配置创建邮件服务，检查 ID 唯一性和默认 ID 有效性
    pub fn from_config(config: &EmailConfig) -> crate::Result<Self> {
        let mut accounts = HashMap::with_capacity(config.accounts.len());

        for account in &config.accounts {
            if accounts.contains_key(&account.id) {
                return Err(duplicate_account_id(&account.id));
            }
            accounts.insert(account.id.clone(), account.clone());
        }

        if !accounts.contains_key(&config.default) {
            return Err(default_not_found(&config.default));
        }

        Ok(Self {
            accounts,
            default_id: config.default.clone(),
        })
    }

    /// 使用默认账户发送 HTML 邮件
    ///
    /// - `to_addr`: 收件人地址
    /// - `subject`: 主题
    /// - `body`: HTML 正文
    /// - `cc`: 抄送地址列表，传 `&[]` 不抄送
    pub async fn send(
        &self,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
    ) -> crate::Result<()> {
        self.send_with(&self.default_id.clone(), to_addr, subject, body, cc)
            .await
    }

    /// 使用指定账户发送 HTML 邮件
    ///
    /// - `id`: 邮箱账户 ID
    /// - `to_addr`: 收件人地址
    /// - `subject`: 主题
    /// - `body`: HTML 正文
    /// - `cc`: 抄送地址列表，传 `&[]` 不抄送
    pub async fn send_with(
        &self,
        id: &str,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
    ) -> crate::Result<()> {
        self.do_send(id, to_addr, subject, body, cc, ContentType::TEXT_HTML)
            .await
    }

    /// 使用默认账户发送 HTML 邮件（语义同 `send`）
    ///
    /// - `to_addr`: 收件人地址
    /// - `subject`: 主题
    /// - `body`: HTML 正文
    /// - `cc`: 抄送地址列表，传 `&[]` 不抄送
    pub async fn send_html(
        &self,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
    ) -> crate::Result<()> {
        self.do_send(
            &self.default_id.clone(),
            to_addr,
            subject,
            body,
            cc,
            ContentType::TEXT_HTML,
        )
        .await
    }

    /// 使用指定账户发送 HTML 邮件（语义同 `send_with`）
    ///
    /// - `id`: 邮箱账户 ID
    /// - `to_addr`: 收件人地址
    /// - `subject`: 主题
    /// - `body`: HTML 正文
    /// - `cc`: 抄送地址列表，传 `&[]` 不抄送
    pub async fn send_html_with(
        &self,
        id: &str,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
    ) -> crate::Result<()> {
        self.do_send(id, to_addr, subject, body, cc, ContentType::TEXT_HTML)
            .await
    }

    /// 使用默认账户发送纯文本邮件
    ///
    /// - `to_addr`: 收件人地址
    /// - `subject`: 主题
    /// - `body`: 纯文本正文
    /// - `cc`: 抄送地址列表，传 `&[]` 不抄送
    pub async fn send_text(
        &self,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
    ) -> crate::Result<()> {
        self.do_send(
            &self.default_id.clone(),
            to_addr,
            subject,
            body,
            cc,
            ContentType::TEXT_PLAIN,
        )
        .await
    }

    /// 使用指定账户发送纯文本邮件
    ///
    /// - `id`: 邮箱账户 ID
    /// - `to_addr`: 收件人地址
    /// - `subject`: 主题
    /// - `body`: 纯文本正文
    /// - `cc`: 抄送地址列表，传 `&[]` 不抄送
    pub async fn send_text_with(
        &self,
        id: &str,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
    ) -> crate::Result<()> {
        self.do_send(id, to_addr, subject, body, cc, ContentType::TEXT_PLAIN)
            .await
    }

    /// 内部发送实现
    async fn do_send(
        &self,
        id: &str,
        to_addr: &str,
        subject: &str,
        body: &str,
        cc: &[&str],
        content_type: ContentType,
    ) -> crate::Result<()> {
        let account = self.accounts.get(id).ok_or_else(|| account_not_found(id))?;

        let transport = Self::build_transport(account)?;

        let from = if account.name.is_empty() {
            account.from.clone()
        } else {
            format!("{} <{}>", account.name, account.from)
        };

        let mut builder = Message::builder()
            .from(
                from.parse()
                    .map_err(|e| send_failed(&format!("Invalid sender address: {}", e)))?,
            )
            .to(to_addr
                .parse()
                .map_err(|e| send_failed(&format!("Invalid recipient address: {}", e)))?)
            .subject(subject);

        for addr in cc {
            builder = builder.cc(addr
                .parse()
                .map_err(|e| send_failed(&format!("Invalid CC address: {}", e)))?);
        }

        let email = builder
            .header(content_type)
            .body(body.to_string())
            .map_err(|e| send_failed(&format!("Failed to build email: {}", e)))?;

        transport.send(email).await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 51002, msg = "Email send failed" },
                "Email send error: {}",
                e
            );
            send_failed(&e.to_string())
        })?;

        Ok(())
    }

    /// 构建 SMTP 传输
    ///
    /// - `secure + !starttls`: 隐式 SSL（端口 465/994）
    /// - `secure + starttls`: STARTTLS（端口 587）
    /// - `!secure`: 无加密
    fn build_transport(
        account: &EmailAccountConfig,
    ) -> crate::Result<AsyncSmtpTransport<Tokio1Executor>> {
        let creds = Credentials::new(account.user.clone(), account.pass.clone());

        #[cfg(feature = "log")]
        debug!(
            host = %account.host,
            port = account.port,
            secure = account.secure,
            starttls = account.starttls,
            "构建 SMTP 传输"
        );

        let transport = if account.secure && account.starttls {
            // STARTTLS 模式（587 端口）
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&account.host)
                .map_err(|e| smtp_connect_failed(&e.to_string()))?
                .port(account.port)
                .credentials(creds)
                .build()
        } else if account.secure {
            // 隐式 SSL 模式（465/994 端口）
            // 使用 builder_dangerous + 手动 Tls::Wrapper，避免 relay() 的端口默认值问题
            let tls_parameters = TlsParameters::builder(account.host.clone())
                .build()
                .map_err(|e| smtp_connect_failed(&format!("TLS 参数构建失败: {}", e)))?;
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&account.host)
                .port(account.port)
                .credentials(creds)
                .tls(Tls::Wrapper(tls_parameters))
                .build()
        } else {
            // 无加密
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&account.host)
                .port(account.port)
                .credentials(creds)
                .build()
        };

        Ok(transport)
    }
}
