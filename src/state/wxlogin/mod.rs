#[allow(dead_code)]
pub mod err;
use err::*;

#[cfg(feature = "wx-login-web")]
mod callback;
#[cfg(feature = "wx-login-web")]
pub use callback::{WxWebLoginCallbackFn, callback as wx_web_login_callback_handler};

#[cfg(any(feature = "wx-login-mini", feature = "wx-login-app"))]
use reqwest::Client;

use serde::Deserialize;

#[cfg(feature = "wx-login-web")]
use crate::state::callbacks::IntoCallback;

#[derive(Clone, Deserialize)]
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
    #[cfg(feature = "wx-login-web")]
    #[serde(skip)]
    pub(crate) web_login_callback: Option<WxWebLoginCallbackFn>,
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

        let response = Client::new()
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50202, msg = "WeChat mini login error" },  "{}", _e);
                mini_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50203, msg = "WeChat mini login error" },  "{}", _e);
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

        let response = Client::new()
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50302, msg = "WeChat APP login error" },  "{}", _e);
                app_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50303, msg = "WeChat APP login error" },  "{}", _e);
                app_response()
            })?;

        // 检查微信API错误
        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64()) {
            if errcode != 0 {
                let errmsg = response
                    .get("errmsg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                #[cfg(feature = "log")]
                tracing::warn!({ code = 40301, msg = "WeChat APP login error" },  "{}", &errmsg);
                return Err(app_api(&errmsg));
            }
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
            self.web_id, &redirect_uri
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
                tracing::error!({ code = 52102, msg = "WeChat web login error" }, "{}", _e);
                web_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52103, msg = "WeChat web login error" }, "{}", _e);
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
                tracing::error!({ code = 52105, msg = "WeChat web refresh token error" }, "{}", _e);
                web_refresh_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52106, msg = "WeChat web refresh token error" }, "{}", _e);
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
                tracing::error!({ code = 52107, msg = "WeChat web check token error" }, "{}", _e);
                web_check_request()
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52108, msg = "WeChat web check token error" }, "{}", _e);
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
                tracing::error!({ code = 52109, msg = "WeChat web userinfo error" }, "{}", _e);
                web_userinfo_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52110, msg = "WeChat web userinfo error" }, "{}", _e);
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
