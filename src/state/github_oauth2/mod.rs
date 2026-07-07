mod callback;
pub mod err;
pub use callback::{GitHubOAuth2CallbackFn, callback as github_oauth2_callback_handler};
use err::*;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::callbacks::IntoCallback;
use crate::state::nonce::Nonce;

// ═══════════════════════════════════════════════════════════════
//  公开类型
// ═══════════════════════════════════════════════════════════════

fn default_scope() -> String {
    "read:user user:email".to_string()
}

fn default_token_url() -> String {
    "https://github.com/login/oauth/access_token".to_string()
}

fn default_user_url() -> String {
    "https://api.github.com/user".to_string()
}

fn default_authorize_url() -> String {
    "https://github.com/login/oauth/authorize".to_string()
}

fn default_callback_path() -> String {
    "auth/github/callback".to_string()
}

fn default_state_expire() -> u64 {
    300
}

#[derive(Clone, Deserialize)]
pub struct GitHubOAuth2 {
    /// GitHub OAuth App 的 Client ID
    pub client_id: String,
    /// GitHub OAuth App 的 Client Secret
    pub client_secret: String,
    /// 服务基础地址, 例如 `http://localhost:5000`
    pub redirect_base: String,
    /// 回调路径, 默认 `auth/github/callback`
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
    /// OAuth2 授权范围, 默认 "read:user user:email"
    #[serde(default = "default_scope")]
    pub scope: String,
    /// GitHub 授权页面 URL
    #[serde(default = "default_authorize_url")]
    pub authorize_url: String,
    /// GitHub Token 交换 URL
    #[serde(default = "default_token_url")]
    pub token_url: String,
    /// GitHub 用户信息 API URL
    #[serde(default = "default_user_url")]
    pub user_url: String,
    #[serde(skip)]
    pub client: reqwest::Client,
    /// 用户注册的回调函数（通过 IntoCallback 自动装箱）
    #[serde(skip)]
    pub(crate) github_oauth2_callback: Option<GitHubOAuth2CallbackFn>,

    // ── State 验证 ──
    /// 是否启用框架内置 state 验证，默认 `false`
    #[serde(default)]
    pub verify_state: bool,
    /// state 有效期（秒），默认 300
    #[serde(default = "default_state_expire")]
    pub state_expire: u64,
}

// ═══════════════════════════════════════════════════════════════
//  GitHub API 响应类型
// ═══════════════════════════════════════════════════════════════

/// GitHub OAuth2 Token 响应
#[derive(Debug, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// GitHub 用户信息
///
/// 字段来源于 GitHub REST API `/user` 端点。
/// `id` 和 `login` 始终存在；其余字段取决于 scope 和用户隐私设置。
/// 未识别的扩展字段通过 `extra` 兜底保留。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    // ── 始终存在 ──
    pub id: i64,
    pub login: String,
    pub node_id: Option<String>,

    // ── 需要 read:user scope ──
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
    pub bio: Option<String>,
    pub company: Option<String>,
    pub blog: Option<String>,
    pub location: Option<String>,
    pub twitter_username: Option<String>,
    pub public_repos: Option<i64>,
    pub public_gists: Option<i64>,
    pub followers: Option<i64>,
    pub following: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,

    // ── 兜底：保留 GitHub 返回的任何额外字段 ──
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// GitHub 用户邮箱信息
#[derive(Debug, Clone, Deserialize)]
pub struct GitHubEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
    pub visibility: Option<String>,
}

/// GitHub OAuth2 登录结果
///
/// 从 `GitHubUser` 提取的结构化数据，字段与 scope 相关：
/// - 始终有值：`github_id`, `username`
/// - `read:user` scope：`display_name`, `avatar_url`, `profile_url`, `bio`, `company`, `blog`, `location`
/// - `read:user user:email` scope：额外包含 `email`
#[derive(Debug, Clone, Serialize, Deserialize, afast::Tag)]
#[tag("GitHub OAuth2 登录结果")]
pub struct GitHubOAuth2Result {
    /// GitHub 用户 ID
    pub github_id: i64,
    /// GitHub 用户名
    pub username: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 邮箱地址
    pub email: Option<String>,
    /// 头像 URL
    pub avatar_url: Option<String>,
    /// GitHub 个人主页 URL
    pub profile_url: Option<String>,
    /// 个人简介
    pub bio: Option<String>,
    /// 公司
    pub company: Option<String>,
    /// 博客/网站
    pub blog: Option<String>,
    /// 所在地
    pub location: Option<String>,
    /// Twitter 用户名
    pub twitter_username: Option<String>,
}

impl From<GitHubUser> for GitHubOAuth2Result {
    fn from(user: GitHubUser) -> Self {
        Self {
            github_id: user.id,
            username: user.login,
            display_name: user.name,
            email: user.email,
            avatar_url: user.avatar_url,
            profile_url: user.html_url,
            bio: user.bio,
            company: user.company,
            blog: user.blog,
            location: user.location,
            twitter_username: user.twitter_username,
        }
    }
}

impl GitHubOAuth2 {
    /// 生成 GitHub OAuth2 授权 URL
    ///
    /// # 参数
    /// - `state`: 可选的 state 参数，用于防止 CSRF 攻击
    ///
    /// # 返回
    /// - 授权 URL 字符串
    ///
    /// 拼接完整的 redirect_uri
    pub fn redirect_uri(&self) -> String {
        format!(
            "{}/{}",
            self.redirect_base.trim_end_matches('/'),
            self.callback_path.trim_start_matches('/')
        )
    }

    pub fn get_authorize_url(&self, state: Option<&str>) -> String {
        let redirect_uri = self.redirect_uri();
        let mut url = format!(
            "{}?client_id={}&redirect_uri={}&scope={}",
            self.authorize_url, self.client_id, redirect_uri, self.scope
        );
        if let Some(s) = state {
            url.push_str(&format!("&state={}", s));
        }
        url
    }

    // ── State 签名 ──

    /// 生成带签名的 state 值
    ///
    /// 格式: `{random}.{timestamp}.{signature}`
    /// 签名: HMAC-SHA256(client_secret, "{random}.{timestamp}")
    pub fn generate_state(&self, nonce: &Nonce) -> String {
        use base64::Engine as _;
        let random = nonce.generate();
        let ts = chrono::Utc::now().timestamp();
        let payload = format!("{}.{}", random, ts);
        let sig = self.hmac_sign(&payload);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(format!("{}.{}", payload, sig))
    }

    /// 生成带内置 state 的授权 URL（推荐）
    ///
    /// 当 `verify_state = true` 时，自动生成签名 state 并拼入 URL。
    /// 当 `verify_state = false` 时，等同于 `get_authorize_url(None)`。
    pub fn generate_authorize_url(&self, nonce: &Nonce) -> String {
        if self.verify_state {
            let state = self.generate_state(nonce);
            self.get_authorize_url(Some(&state))
        } else {
            self.get_authorize_url(None)
        }
    }

    /// 验证 state 签名是否合法且未过期
    pub fn validate_state(&self, state: &str) -> bool {
        use base64::Engine as _;
        let decoded = match base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(state) {
            Ok(d) => String::from_utf8_lossy(&d).to_string(),
            Err(_) => return false,
        };
        let parts: Vec<&str> = decoded.splitn(3, '.').collect();
        if parts.len() != 3 {
            return false;
        }
        let (random, ts_str, sig) = (parts[0], parts[1], parts[2]);
        let payload = format!("{}.{}", random, ts_str);
        if self.hmac_sign(&payload) != sig {
            return false;
        }
        if let Ok(ts) = ts_str.parse::<i64>() {
            chrono::Utc::now().timestamp() - ts <= self.state_expire as i64
        } else {
            false
        }
    }

    fn hmac_sign(&self, data: &str) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.client_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        let result = mac.finalize().into_bytes();
        result.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// 使用授权码交换 Access Token
    ///
    /// # 参数
    /// - `code`: GitHub 回调返回的授权码
    ///
    /// # 返回
    /// - `crate::Result<GitHubTokenResponse>`
    pub async fn exchange_code(&self, code: &str) -> crate::Result<GitHubTokenResponse> {
        let response = self
            .client
            .post(&self.token_url)
            .header("Accept", "application/json")
            .json(&serde_json::json!({
                "client_id": self.client_id,
                "client_secret": self.client_secret,
                "code": code,
                "redirect_uri": self.redirect_uri(),
            }))
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50501, msg = "GitHub OAuth2 error: Token request failed" },
                    "{}", _e
                );
                exchange_request()
            })?;

        let token_response: GitHubTokenResponse = response.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 50502, msg = "GitHub OAuth2 error: Token response parse failed" },
                "{}", _e
            );
            exchange_parse()
        })?;

        if token_response.error.is_some() {
            #[cfg(feature = "log")]
            tracing::warn!(
                { code = 40501, msg = "GitHub OAuth2 error: Token exchange failed" },
                "error: {:?}, description: {:?}",
                token_response.error,
                token_response.error_description
            );
            return Err(exchange_auth());
        }

        Ok(token_response)
    }

    /// 使用 Access Token 获取用户信息
    ///
    /// # 参数
    /// - `access_token`: GitHub Access Token
    ///
    /// # 返回
    /// - `crate::Result<GitHubUser>`
    pub async fn get_user(&self, access_token: &str) -> crate::Result<GitHubUser> {
        let response = self
            .client
            .get(&self.user_url)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .header("User-Agent", "afaster-github-oauth2")
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50503, msg = "GitHub OAuth2 error: Failed to get user info" },
                    "{}", _e
                );
                user_request()
            })?;

        let user: GitHubUser = response.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 50504, msg = "GitHub OAuth2 error: User info parse failed" },
                "{}", _e
            );
            user_parse()
        })?;

        Ok(user)
    }

    /// 获取用户的邮箱列表 (需要 user:email scope)
    ///
    /// # 参数
    /// - `access_token`: GitHub Access Token
    ///
    /// # 返回
    /// - `crate::Result<Vec<GitHubEmail>>`
    pub async fn get_user_emails(&self, access_token: &str) -> crate::Result<Vec<GitHubEmail>> {
        let response = self
            .client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Accept", "application/json")
            .header("User-Agent", "afaster-github-oauth2")
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50505, msg = "GitHub OAuth2 error: Failed to get user email" },
                    "{}", _e
                );
                email_request()
            })?;

        let emails: Vec<GitHubEmail> = response.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 50506, msg = "GitHub OAuth2 error: User email parse failed" },
                "{}", _e
            );
            email_parse()
        })?;

        Ok(emails)
    }

    /// 完整的 GitHub OAuth2 登录流程
    ///
    /// # 参数
    /// - `code`: GitHub 回调返回的授权码
    ///
    /// # 返回
    /// - `crate::Result<GitHubOAuth2Result>` 包含用户信息的登录结果
    pub async fn login(&self, code: &str) -> crate::Result<GitHubOAuth2Result> {
        // 1. 交换授权码获取 Token
        let token_response = self.exchange_code(code).await?;

        let access_token = token_response.access_token.ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::warn!({ code = 40502, msg = "GitHub OAuth2 error: No access_token obtained" });
            no_access_token()
        })?;

        // 2. 获取用户信息
        let user = self.get_user(&access_token).await?;

        // 3. 如果用户没有公开邮箱, 尝试获取邮箱列表
        let email = if user.email.is_none() {
            match self.get_user_emails(&access_token).await {
                Ok(emails) => emails
                    .iter()
                    .find(|e| e.primary && e.verified)
                    .map(|e| e.email.clone())
                    .or_else(|| emails.iter().find(|e| e.verified).map(|e| e.email.clone())),
                Err(_) => None,
            }
        } else {
            user.email.clone()
        };

        let mut result = GitHubOAuth2Result::from(user);
        result.email = email;

        Ok(result)
    }

    /// 设置 GitHub OAuth2 回调函数
    pub fn with_github_callback(
        mut self,
        cb: impl IntoCallback<
            (crate::AppState, GitHubOAuth2Result, Option<String>),
            crate::Result<GitHubOAuth2Result>,
        >,
    ) -> Self {
        self.github_oauth2_callback = Some(cb.into_callback());
        self
    }

    /// 是否启用了内置 state 验证
    pub fn is_verify_state(&self) -> bool {
        self.verify_state
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "github_oauth2")?;
        instance.client = super::default_http_client();
        Ok(instance)
    }
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster GitHub OAuth2 配置扩展
pub trait AFasterGitHubOAuth2Ext {
    /// 链式配置 GitHub OAuth2
    fn with_github_oauth2(self, f: impl FnOnce(GitHubOAuth2) -> GitHubOAuth2) -> Self;
}

impl AFasterGitHubOAuth2Ext for crate::AFaster {
    fn with_github_oauth2(mut self, f: impl FnOnce(GitHubOAuth2) -> GitHubOAuth2) -> Self {
        self.state.github_oauth2 = f(self.state.github_oauth2);
        self
    }
}
