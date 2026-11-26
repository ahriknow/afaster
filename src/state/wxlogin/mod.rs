#[allow(dead_code)]
pub mod err;
use err::*;

#[cfg(feature = "wx-login-web")]
mod callback;
#[cfg(feature = "wx-login-web")]
pub use callback::{WxWebLoginCallbackFn, callback as wx_web_login_callback_handler};

use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "log")]
use super::log_request_error;

#[cfg(feature = "wx-login-web")]
use crate::state::callbacks::IntoCallback;

#[derive(Clone, Deserialize)]
#[serde(default)]
pub struct WxLogin {
    #[cfg(feature = "wx-login-mini")]
    pub mini_id: String,
    #[cfg(feature = "wx-login-mini")]
    pub mini_secret: String,
    #[cfg(feature = "wx-login-app")]
    pub app_id: String,
    #[cfg(feature = "wx-login-app")]
    pub app_secret: String,
    #[cfg(feature = "wx-login-web")]
    pub web_id: String,
    #[cfg(feature = "wx-login-web")]
    pub web_secret: String,
    #[cfg(feature = "wx-login-web")]
    pub redirect_base: String,
    #[cfg(feature = "wx-login-web")]
    #[serde(default = "default_web_callback_path")]
    pub callback_path: String,
    #[cfg(any(
        feature = "wx-login-mini",
        feature = "wx-login-app",
        feature = "wx-login-web"
    ))]
    #[serde(skip)]
    pub client: reqwest::Client,
    #[cfg(feature = "wx-login-mini")]
    #[serde(skip)]
    mini_access_token: Arc<Mutex<Option<CachedMiniAccessToken>>>,
    #[cfg(feature = "wx-login-web")]
    #[serde(skip)]
    pub(crate) web_login_callback: Option<WxWebLoginCallbackFn>,
}

impl Default for WxLogin {
    fn default() -> Self {
        Self {
            #[cfg(feature = "wx-login-mini")]
            mini_id: String::new(),
            #[cfg(feature = "wx-login-mini")]
            mini_secret: String::new(),
            #[cfg(feature = "wx-login-app")]
            app_id: String::new(),
            #[cfg(feature = "wx-login-app")]
            app_secret: String::new(),
            #[cfg(feature = "wx-login-web")]
            web_id: String::new(),
            #[cfg(feature = "wx-login-web")]
            web_secret: String::new(),
            #[cfg(feature = "wx-login-web")]
            redirect_base: String::new(),
            #[cfg(feature = "wx-login-web")]
            callback_path: default_web_callback_path(),
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web"
            ))]
            client: super::default_http_client(),
            #[cfg(feature = "wx-login-mini")]
            mini_access_token: Arc::new(Mutex::new(None)),
            #[cfg(feature = "wx-login-web")]
            web_login_callback: None,
        }
    }
}

#[cfg(feature = "wx-login-mini")]
struct CachedMiniAccessToken {
    value: String,
    expires_at: std::time::Instant,
}

#[cfg(feature = "wx-login-web")]
fn default_web_callback_path() -> String {
    "auth/wechat/callback".to_string()
}

// 微信小程序登录响应
#[cfg(feature = "wx-login-mini")]
#[derive(Debug, serde::Deserialize)]
pub struct MiniLoginResponse {
    pub openid: String,
    pub session_key: String,
    pub unionid: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

#[cfg(feature = "wx-login-mini")]
#[derive(Debug, Deserialize)]
pub struct WxMiniPhoneNumber {
    #[serde(rename = "phoneNumber")]
    pub phone_number: String,
    #[serde(rename = "purePhoneNumber")]
    pub pure_phone_number: String,
    #[serde(rename = "countryCode")]
    pub country_code: String,
}

#[cfg(feature = "wx-login-mini")]
#[derive(Debug, Deserialize)]
struct WxMiniPhoneNumberResponse {
    #[serde(default)]
    errcode: Option<i32>,
    #[serde(default)]
    errmsg: Option<String>,
    phone_info: Option<WxMiniPhoneNumber>,
}

#[cfg(feature = "wx-login-mini")]
#[derive(Debug, Deserialize)]
struct WxMiniAccessTokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

// 微信APP登录响应
#[cfg(feature = "wx-login-app")]
#[derive(Debug, serde::Deserialize)]
pub struct AppLoginResponse {
    pub access_token: String,
    pub expires_in: i32,
    pub refresh_token: String,
    pub openid: String,
    pub scope: String,
    pub unionid: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

impl WxLogin {
    #[cfg(feature = "wx-login-mini")]
    async fn mini_access_token(&self) -> crate::Result<String> {
        let mut cache = self.mini_access_token.lock().await;
        if let Some(token) = cache.as_ref()
            && token.expires_at > std::time::Instant::now()
        {
            return Ok(token.value.clone());
        }

        let mut url = reqwest::Url::parse("https://api.weixin.qq.com/cgi-bin/token")
            .map_err(|_| mini_phone_token_url())?;
        url.query_pairs_mut()
            .append_pair("grant_type", "client_credential")
            .append_pair("appid", &self.mini_id)
            .append_pair("secret", &self.mini_secret);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50206, "WeChat mini access token request failed");
                mini_phone_token_request()
            })?
            .json::<WxMiniAccessTokenResponse>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50207, "WeChat mini access token response parse failed");
                mini_phone_token_response()
            })?;
        if response.errcode.unwrap_or_default() != 0 {
            return Err(mini_api(
                response
                    .errmsg
                    .as_deref()
                    .unwrap_or("WeChat access token request failed"),
            ));
        }
        let value = response
            .access_token
            .ok_or_else(mini_phone_token_response)?;
        let expires_in = response.expires_in.unwrap_or_default();
        if value.is_empty() || expires_in == 0 {
            return Err(mini_phone_token_response());
        }

        *cache = Some(CachedMiniAccessToken {
            value: value.clone(),
            expires_at: std::time::Instant::now()
                + std::time::Duration::from_secs(expires_in.saturating_sub(120).max(1)),
        });
        Ok(value)
    }

    #[cfg(feature = "wx-login-mini")]
    /// 微信小程序登录
    ///
    /// # 参数
    /// - `code`: 前端获取的登录code
    ///
    /// # 返回
    /// - `crate::Result<MiniLoginResponse>`
    pub async fn mini_login(&self, code: &str) -> crate::Result<MiniLoginResponse> {
        // 使用 url crate 构建带查询参数的 URL
        let mut url =
            reqwest::Url::parse("https://api.weixin.qq.com/sns/jscode2session").map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50201, msg = "WeChat mini login error" },  "{}", _e);
                mini_url()
            })?;
        url.query_pairs_mut()
            .append_pair("appid", &self.mini_id)
            .append_pair("secret", &self.mini_secret)
            .append_pair("js_code", code)
            .append_pair("grant_type", "authorization_code");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50202, "WeChat mini login request failed");
                mini_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50203, "WeChat mini login response parse failed");
                mini_response()
            })?;

        // 检查微信API错误
        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = response
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 40201, msg = "WeChat mini login error" },  "{}", &errmsg);
            return Err(mini_api(&errmsg));
        }

        let login_response: MiniLoginResponse = serde_json::from_value(response).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50204, msg = "WeChat mini login error" },  "{}", _e);
            mini_parse()
        })?;
        Ok(login_response)
    }

    #[cfg(feature = "wx-login-mini")]
    /// 用小程序手机号授权 code 换取手机号信息。
    ///
    /// access_token 在当前 WxLogin 实例内缓存，并在过期前刷新。
    pub async fn get_user_phone_number(&self, code: &str) -> crate::Result<WxMiniPhoneNumber> {
        if code.trim().is_empty() {
            return Err(mini_phone_code_missing());
        }
        let access_token = self.mini_access_token().await?;
        let response = self
            .client
            .post("https://api.weixin.qq.com/wxa/business/getuserphonenumber")
            .query(&[("access_token", access_token)])
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50208, "WeChat mini phone number request failed");
                mini_phone_request()
            })?
            .json::<WxMiniPhoneNumberResponse>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50209, "WeChat mini phone number response parse failed");
                mini_phone_response()
            })?;
        if response.errcode.unwrap_or_default() != 0 {
            return Err(mini_api(
                response
                    .errmsg
                    .as_deref()
                    .unwrap_or("WeChat phone number request failed"),
            ));
        }
        response.phone_info.ok_or_else(mini_phone_parse)
    }

    #[cfg(feature = "wx-login-app")]
    /// 微信APP登录
    ///
    /// # 参数
    /// - `code`: 前端获取的登录code
    ///
    /// # 返回
    /// - `crate::Result<AppLoginResponse>`
    pub async fn app_login(&self, code: &str) -> crate::Result<AppLoginResponse> {
        // 使用 url crate 构建带查询参数的 URL
        let mut url = reqwest::Url::parse("https://api.weixin.qq.com/sns/oauth2/access_token")
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50301, msg = "WeChat APP login error" },  "{}", _e);
                app_url()
            })?;
        url.query_pairs_mut()
            .append_pair("appid", &self.app_id)
            .append_pair("secret", &self.app_secret)
            .append_pair("code", code)
            .append_pair("grant_type", "authorization_code");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50302, "WeChat app login request failed");
                app_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 50303, "WeChat app login response parse failed");
                app_response()
            })?;

        // 检查微信API错误
        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = response
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 40301, msg = "WeChat APP login error" },  "{}", &errmsg);
            return Err(app_api(&errmsg));
        }

        let login_response: AppLoginResponse = serde_json::from_value(response).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50304, msg = "WeChat APP login error" },  "{}", _e);
            app_parse()
        })?;
        Ok(login_response)
    }

    // ═══════════════════════════════════════════════════════════════
    //  微信网页扫码登录
    // ═══════════════════════════════════════════════════════════════

    #[cfg(feature = "wx-login-web")]
    /// 拼接完整的 redirect_uri
    pub fn web_redirect_uri(&self) -> String {
        format!(
            "{}/{}",
            self.redirect_base.trim_end_matches('/'),
            self.callback_path.trim_start_matches('/')
        )
    }

    #[cfg(feature = "wx-login-web")]
    /// 生成微信网页扫码登录授权 URL
    ///
    /// 前端跳转此 URL 后，用户扫码授权，微信会重定向到 redirect_uri 并携带 code 和 state。
    ///
    /// # 参数
    /// - `state`: 可选的 state 参数，用于防止 CSRF 攻击
    ///
    /// # 返回
    /// - 授权 URL 字符串
    pub fn get_authorize_url(&self, state: Option<&str>) -> String {
        let redirect_uri = self.web_redirect_uri();
        let mut url = format!(
            "https://open.weixin.qq.com/connect/qrconnect?appid={}&redirect_uri={}&response_type=code&scope=snsapi_login",
            self.web_id, redirect_uri
        );
        if let Some(s) = state {
            url.push_str(&format!("&state={}", s));
        }
        url.push_str("#wechat_redirect");
        url
    }

    #[cfg(feature = "wx-login-web")]
    /// 通过 code 换取 access_token
    ///
    /// 对应微信 API: `https://api.weixin.qq.com/sns/oauth2/access_token`
    ///
    /// # 参数
    /// - `code`: 授权码（回调 URL 中的 code 参数）
    ///
    /// # 返回
    /// - `crate::Result<WxWebAccessTokenResponse>`
    pub async fn web_login(&self, code: &str) -> crate::Result<WxWebAccessTokenResponse> {
        let mut url = reqwest::Url::parse("https://api.weixin.qq.com/sns/oauth2/access_token")
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52101, msg = "WeChat web login error" }, "{}", _e);
                web_url()
            })?;
        url.query_pairs_mut()
            .append_pair("appid", &self.web_id)
            .append_pair("secret", &self.web_secret)
            .append_pair("code", code)
            .append_pair("grant_type", "authorization_code");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52102, "WeChat web login request failed");
                web_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52103, "WeChat web login response parse failed");
                web_response()
            })?;

        // 检查微信API错误
        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = response
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 42101, msg = "WeChat web login error" }, "{}", &errmsg);
            return Err(web_api(&errmsg));
        }

        let token_response: WxWebAccessTokenResponse =
            serde_json::from_value(response).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52104, msg = "WeChat web login error" }, "{}", _e);
                web_parse()
            })?;
        Ok(token_response)
    }

    #[cfg(feature = "wx-login-web")]
    /// 刷新 access_token
    ///
    /// access_token 有效期 2 小时，refresh_token 有效期 30 天。
    ///
    /// # 参数
    /// - `refresh_token`: 刷新令牌
    ///
    /// # 返回
    /// - `crate::Result<WxWebAccessTokenResponse>`
    pub async fn web_refresh_token(
        &self,
        refresh_token: &str,
    ) -> crate::Result<WxWebAccessTokenResponse> {
        let mut url = reqwest::Url::parse("https://api.weixin.qq.com/sns/oauth2/refresh_token")
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52105, msg = "WeChat web refresh token error" }, "{}", _e);
                web_refresh_request()
            })?;
        url.query_pairs_mut()
            .append_pair("appid", &self.web_id)
            .append_pair("grant_type", "refresh_token")
            .append_pair("refresh_token", refresh_token);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52105, "WeChat web refresh token request failed");
                web_refresh_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52106, "WeChat web refresh token response parse failed");
                web_refresh_response()
            })?;

        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = response
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 42101, msg = "WeChat web refresh token error" }, "{}", &errmsg);
            return Err(web_api(&errmsg));
        }

        let token_response: WxWebAccessTokenResponse =
            serde_json::from_value(response).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52106, msg = "WeChat web refresh token error" }, "{}", _e);
                web_refresh_response()
            })?;
        Ok(token_response)
    }

    #[cfg(feature = "wx-login-web")]
    /// 检验 access_token 是否有效
    ///
    /// # 参数
    /// - `access_token`: 接口调用凭证
    /// - `openid`: 用户唯一标识
    ///
    /// # 返回
    /// - `crate::Result<bool>` true 表示有效
    pub async fn web_check_token(&self, access_token: &str, openid: &str) -> crate::Result<bool> {
        let mut url = reqwest::Url::parse("https://api.weixin.qq.com/sns/auth").map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52107, msg = "WeChat web check token error" }, "{}", _e);
            web_check_request()
        })?;
        url.query_pairs_mut()
            .append_pair("access_token", access_token)
            .append_pair("openid", openid);

        let response: serde_json::Value = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52107, "WeChat web check token request failed");
                web_check_request()
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52108, "WeChat web check token response parse failed");
                web_check_response()
            })?;

        let errcode = response
            .get("errcode")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        Ok(errcode == 0)
    }

    #[cfg(feature = "wx-login-web")]
    /// 获取用户个人信息
    ///
    /// 需要用户已授权 `snsapi_userinfo` scope。
    ///
    /// # 参数
    /// - `access_token`: 接口调用凭证
    /// - `openid`: 用户唯一标识
    /// - `lang`: 语言版本，可选 `zh_CN` / `zh_TW` / `en`，默认 `en`
    ///
    /// # 返回
    /// - `crate::Result<WxWebUserInfo>`
    pub async fn web_get_userinfo(
        &self,
        access_token: &str,
        openid: &str,
        lang: Option<&str>,
    ) -> crate::Result<WxWebUserInfo> {
        let mut url =
            reqwest::Url::parse("https://api.weixin.qq.com/sns/userinfo").map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52109, msg = "WeChat web userinfo error" }, "{}", _e);
                web_userinfo_request()
            })?;
        url.query_pairs_mut()
            .append_pair("access_token", access_token)
            .append_pair("openid", openid)
            .append_pair("lang", lang.unwrap_or("zh_CN"));

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52109, "WeChat web userinfo request failed");
                web_userinfo_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                log_request_error(_e, 52110, "WeChat web userinfo response parse failed");
                web_userinfo_response()
            })?;

        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = response
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 42101, msg = "WeChat web userinfo error" }, "{}", &errmsg);
            return Err(web_api(&errmsg));
        }

        let user_info: WxWebUserInfo = serde_json::from_value(response).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52110, msg = "WeChat web userinfo error" }, "{}", _e);
            web_userinfo_response()
        })?;
        Ok(user_info)
    }

    #[cfg(feature = "wx-login-web")]
    /// 完整的微信网页扫码登录流程（仅换取 access_token）
    ///
    /// # 参数
    /// - `code`: 回调 URL 中的授权码
    ///
    /// # 返回
    /// - `crate::Result<WxWebLoginResult>` 包含 token 信息和用户 openid
    pub async fn web_login_full(&self, code: &str) -> crate::Result<WxWebLoginResult> {
        let token = self.web_login(code).await?;
        let userinfo = self
            .web_get_userinfo(&token.access_token, &token.openid, None)
            .await
            .ok(); // userinfo 可能因 scope 不足而失败，忽略

        Ok(WxWebLoginResult {
            access_token: token.access_token,
            expires_in: token.expires_in,
            refresh_token: token.refresh_token,
            openid: token.openid,
            scope: token.scope,
            unionid: token.unionid,
            userinfo,
        })
    }

    #[cfg(feature = "wx-login-web")]
    /// 设置网页登录回调函数
    pub fn with_web_callback(
        mut self,
        cb: impl IntoCallback<
            (crate::AppState, WxWebLoginResult, Option<String>),
            crate::Result<WxWebLoginResult>,
        >,
    ) -> Self {
        self.web_login_callback = Some(cb.into_callback());
        self
    }
}

// ═══════════════════════════════════════════════════════════════
//  微信网页登录响应类型
// ═══════════════════════════════════════════════════════════════

/// 通过 code 换取的 access_token 响应
#[cfg(any(feature = "wx-login-web", feature = "wx-official"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WxWebAccessTokenResponse {
    /// 接口调用凭证
    pub access_token: String,
    /// access_token 有效期（秒），目前为 7200
    pub expires_in: i32,
    /// 用户刷新 access_token
    pub refresh_token: String,
    /// 授权用户唯一标识
    pub openid: String,
    /// 用户授权的作用域
    pub scope: String,
    /// 用户统一标识（仅当已获 userinfo 授权时返回）
    pub unionid: Option<String>,
}

/// 微信网页登录用户信息
#[cfg(any(feature = "wx-login-web", feature = "wx-official"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, afast::Tag)]
#[tag("微信网页登录用户信息")]
pub struct WxWebUserInfo {
    /// 普通用户的标识
    pub openid: String,
    /// 用户昵称
    pub nickname: Option<String>,
    /// 用户性别：1=男性，2=女性
    pub sex: Option<i32>,
    /// 省份
    pub province: Option<String>,
    /// 城市
    pub city: Option<String>,
    /// 国家
    pub country: Option<String>,
    /// 用户头像 URL
    pub headimgurl: Option<String>,
    /// 用户特权信息
    pub privilege: Option<Vec<String>>,
    /// 用户统一标识
    pub unionid: Option<String>,
}

/// 微信网页扫码登录结果
///
/// 整合 access_token 信息和用户信息，供回调使用。
/// 网页扫码登录和公众号网页授权共用此类型。
#[cfg(any(feature = "wx-login-web", feature = "wx-official"))]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, afast::Tag)]
#[tag("微信网页登录结果")]
pub struct WxWebLoginResult {
    /// 接口调用凭证
    pub access_token: String,
    /// access_token 有效期（秒）
    pub expires_in: i32,
    /// 用户刷新 access_token
    pub refresh_token: String,
    /// 授权用户唯一标识
    pub openid: String,
    /// 用户授权的作用域
    pub scope: String,
    /// 用户统一标识
    pub unionid: Option<String>,
    /// 用户信息（仅当 scope 包含 snsapi_userinfo 时有值）
    pub userinfo: Option<WxWebUserInfo>,
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 微信登录配置扩展
pub trait AFasterWxLoginExt {
    /// 链式配置微信登录
    fn with_wxlogin(self, f: impl FnOnce(WxLogin) -> WxLogin) -> Self;
}

impl AFasterWxLoginExt for crate::AFaster {
    fn with_wxlogin(mut self, f: impl FnOnce(WxLogin) -> WxLogin) -> Self {
        self.state.wxlogin = f(self.state.wxlogin);
        self
    }
}

impl WxLogin {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let map = table
            .get("wxlogin")
            .and_then(toml::Value::as_table)
            .cloned()
            .unwrap_or_default();
        Ok(Self {
            #[cfg(feature = "wx-login-mini")]
            mini_id: match (map.get("mini_id"), std::env::var("WXLOGIN_MINI_ID")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].mini_id must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].mini_id or WXLOGIN_MINI_ID",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_MINI_ID is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-mini")]
            mini_secret: match (map.get("mini_secret"), std::env::var("WXLOGIN_MINI_SECRET")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].mini_secret must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].mini_secret or WXLOGIN_MINI_SECRET",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_MINI_SECRET is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-app")]
            app_id: match (map.get("app_id"), std::env::var("WXLOGIN_APP_ID")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].app_id must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].app_id or WXLOGIN_APP_ID",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_APP_ID is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-app")]
            app_secret: match (map.get("app_secret"), std::env::var("WXLOGIN_APP_SECRET")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].app_secret must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].app_secret or WXLOGIN_APP_SECRET",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_APP_SECRET is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-web")]
            web_id: match (map.get("web_id"), std::env::var("WXLOGIN_WEB_ID")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].web_id must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].web_id or WXLOGIN_WEB_ID",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_WEB_ID is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-web")]
            web_secret: match (map.get("web_secret"), std::env::var("WXLOGIN_WEB_SECRET")) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].web_secret must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].web_secret or WXLOGIN_WEB_SECRET",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_WEB_SECRET is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-web")]
            redirect_base: match (
                map.get("redirect_base"),
                std::env::var("WXLOGIN_REDIRECT_BASE"),
            ) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].redirect_base must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => {
                    return Err(crate::Error::custom(
                        50001,
                        "missing required configuration [wxlogin].redirect_base or WXLOGIN_REDIRECT_BASE",
                    ));
                }
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_REDIRECT_BASE is not valid UTF-8",
                    ));
                }
            },
            #[cfg(feature = "wx-login-web")]
            callback_path: match (
                map.get("callback_path"),
                std::env::var("WXLOGIN_CALLBACK_PATH"),
            ) {
                (_, Ok(value)) => value,
                (Some(value), Err(std::env::VarError::NotPresent)) => value
                    .as_str()
                    .ok_or_else(|| {
                        crate::Error::custom(50001, "[wxlogin].callback_path must be a string")
                    })?
                    .to_owned(),
                (None, Err(std::env::VarError::NotPresent)) => default_web_callback_path(),
                (_, Err(std::env::VarError::NotUnicode(_))) => {
                    return Err(crate::Error::custom(
                        50001,
                        "WXLOGIN_CALLBACK_PATH is not valid UTF-8",
                    ));
                }
            },
            #[cfg(any(
                feature = "wx-login-mini",
                feature = "wx-login-app",
                feature = "wx-login-web"
            ))]
            client: super::default_http_client(),
            #[cfg(feature = "wx-login-mini")]
            mini_access_token: Arc::new(Mutex::new(None)),
            #[cfg(feature = "wx-login-web")]
            web_login_callback: None,
        })
    }
}
