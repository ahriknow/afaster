pub mod err;
use err::*;

#[cfg(feature = "wx-official")]
mod callback_mp;
#[cfg(feature = "wx-official")]
pub use callback_mp::{WxMpLoginCallbackFn, callback as wx_mp_login_callback_handler};

use crate::state::callbacks::IntoCallback;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

fn default_base_url() -> String {
    "https://api.weixin.qq.com".to_string()
}

#[derive(Clone, Deserialize)]
pub struct WxOfficial {
    pub app_id: String,
    pub app_secret: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// 公众号网页授权回调基础地址 (wx-login-mp)
    #[serde(default)]
    pub redirect_base: String,
    /// 公众号网页授权回调路径 (wx-login-mp)
    #[serde(default = "default_mp_callback_path")]
    pub callback_path: String,
    #[serde(skip)]
    pub client: reqwest::Client,
    #[serde(skip)]
    token_cache: Arc<Mutex<Option<CachedToken>>>,
    #[serde(skip)]
    pub(crate) mp_login_callback: Option<WxMpLoginCallbackFn>,
}

fn default_mp_callback_path() -> String {
    "auth/wechat/mp/callback".to_string()
}

#[derive(Clone)]
struct CachedToken {
    access_token: String,
    expires_at: std::time::Instant,
}

// ═══════════════════════════════════════════════════════════════
//  公共响应类型
// ═══════════════════════════════════════════════════════════════

fn check_error(value: &serde_json::Value) -> crate::Result<()> {
    if let Some(errcode) = value.get("errcode").and_then(|v| v.as_i64())
        && errcode != 0
    {
        let errmsg = value
            .get("errmsg")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error")
            .to_string();
        return Err(api_error(&errmsg));
    }
    Ok(())
}

/// 判断是否为 token 过期错误
fn is_token_error(value: &serde_json::Value) -> bool {
    matches!(
        value.get("errcode").and_then(|v| v.as_i64()),
        Some(40001 | 42001)
    )
}

// ═══════════════════════════════════════════════════════════════
//  access_token 管理
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    expires_in: Option<i64>,
}

impl WxOfficial {
    /// 获取 access_token（自动缓存，过期前 5 分钟刷新）
    pub async fn get_access_token(&self) -> crate::Result<String> {
        {
            let cache = self.token_cache.lock().await;
            if let Some(ref cached) = *cache
                && cached.expires_at > std::time::Instant::now()
            {
                return Ok(cached.access_token.clone());
            }
        }
        self.refresh_access_token(false).await
    }

    /// 强制刷新 access_token
    pub async fn refresh_access_token(&self, force: bool) -> crate::Result<String> {
        let url = format!("{}/cgi-bin/stable_token", self.base_url);

        let body = serde_json::json!({
            "grant_type": "client_credential",
            "appid": self.app_id,
            "secret": self.app_secret,
            "force_refresh": force,
        });

        let resp: serde_json::Value = if force {
            self.client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!({ code = 52301, msg = "access_token error" }, "{}", _e);
                    token_request()
                })?
                .json()
                .await
                .map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!({ code = 52302, msg = "access_token error" }, "{}", _e);
                    token_response()
                })?
        } else {
            self.client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!({ code = 52301, msg = "access_token error" }, "{}", _e);
                    token_request()
                })?
                .json()
                .await
                .map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!({ code = 52302, msg = "access_token error" }, "{}", _e);
                    token_response()
                })?
        };

        check_error(&resp)?;

        let token_resp: TokenResponse = serde_json::from_value(resp).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52302, msg = "access_token error" }, "{}", _e);
            token_response()
        })?;

        let access_token = token_resp.access_token.ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52302, msg = "access_token error: missing token" });
            token_response()
        })?;

        let expires_in = token_resp.expires_in.unwrap_or(7200);
        // 提前 5 分钟刷新
        let expires_at = std::time::Instant::now()
            + std::time::Duration::from_secs((expires_in - 300).max(60) as u64);

        let mut cache = self.token_cache.lock().await;
        *cache = Some(CachedToken {
            access_token: access_token.clone(),
            expires_at,
        });

        Ok(access_token)
    }

    /// 执行 API 调用，自动处理 token 过期被动刷新
    ///
    /// 如果响应 errcode 为 40001 或 42001，自动强制刷新 token 并重试一次。
    async fn do_api<F, Fut>(&self, make_request: F) -> crate::Result<serde_json::Value>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = crate::Result<serde_json::Value>>,
    {
        let token = self.get_access_token().await?;
        let resp = make_request(token).await?;

        // 被动刷新：40001/42001 表示 token 过期
        if is_token_error(&resp) {
            #[cfg(feature = "log")]
            tracing::warn!("wx_official token expired, refreshing...");
            self.token_cache.lock().await.take();
            let token = self.refresh_access_token(true).await?;
            return make_request(token).await;
        }

        Ok(resp)
    }

    // ═══════════════════════════════════════════════════════════════
    //  模板消息
    // ═══════════════════════════════════════════════════════════════

    /// 发送模板消息（自动处理 token 过期重试）
    pub async fn send_template_message(
        &self,
        touser: &str,
        template_id: &str,
        data: serde_json::Value,
        url: Option<&str>,
        miniprogram: Option<serde_json::Value>,
    ) -> crate::Result<i64> {
        let touser = touser.to_string();
        let template_id = template_id.to_string();
        let url = url.map(|s| s.to_string());
        let base_url = self.base_url.clone();
        let client = self.client.clone();

        let resp = self
            .do_api(move |token| {
                let touser = touser.clone();
                let template_id = template_id.clone();
                let data = data.clone();
                let url = url.clone();
                let miniprogram = miniprogram.clone();
                let base_url = base_url.clone();
                let client = client.clone();
                async move {
                    let api_url = format!(
                        "{}/cgi-bin/message/template/send?access_token={}",
                        base_url, token
                    );
                    let mut body = serde_json::json!({
                        "touser": touser,
                        "template_id": template_id,
                        "data": data,
                    });
                    if let Some(u) = url {
                        body["url"] = serde_json::Value::String(u);
                    }
                    if let Some(mp) = miniprogram {
                        body["miniprogram"] = mp;
                    }
                    let resp: serde_json::Value = client
                    .post(&api_url)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52303, msg = "template send error" }, "{}", _e);
                        template_send_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52304, msg = "template send error" }, "{}", _e);
                        template_send_response()
                    })?;
                    check_error(&resp)?;
                    Ok(resp)
                }
            })
            .await?;

        let msgid = resp.get("msgid").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(msgid)
    }

    // ═══════════════════════════════════════════════════════════════
    //  自定义菜单
    // ═══════════════════════════════════════════════════════════════

    /// 创建自定义菜单（自动处理 token 过期重试）
    pub async fn create_menu(&self, buttons: serde_json::Value) -> crate::Result<()> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            let buttons = buttons.clone();
            async move {
                let url = format!("{}/cgi-bin/menu/create?access_token={}", base_url, token);
                let resp: serde_json::Value = client
                    .post(&url)
                    .json(&serde_json::json!({ "button": buttons }))
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52305, msg = "menu create error" }, "{}", _e);
                        menu_create_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52306, msg = "menu create error" }, "{}", _e);
                        menu_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await?;
        Ok(())
    }

    /// 获取自定义菜单配置（API 设置的）
    pub async fn get_menu(&self) -> crate::Result<serde_json::Value> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            async move {
                let url = format!("{}/cgi-bin/menu/get?access_token={}", base_url, token);
                let resp: serde_json::Value = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52310, msg = "menu get error" }, "{}", _e);
                        menu_get_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52306, msg = "menu get error" }, "{}", _e);
                        menu_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await
    }

    /// 查询当前菜单配置（包含官网设置的）
    pub async fn get_current_selfmenu_info(&self) -> crate::Result<serde_json::Value> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            async move {
                let url = format!(
                    "{}/cgi-bin/get_current_selfmenu_info?access_token={}",
                    base_url, token
                );
                let resp: serde_json::Value = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52310, msg = "selfmenu info error" }, "{}", _e);
                        menu_get_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52306, msg = "selfmenu info error" }, "{}", _e);
                        menu_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await
    }

    /// 删除自定义菜单（含全部个性化菜单）
    pub async fn delete_menu(&self) -> crate::Result<()> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            async move {
                let url = format!("{}/cgi-bin/menu/delete?access_token={}", base_url, token);
                let resp: serde_json::Value = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52307, msg = "menu delete error" }, "{}", _e);
                        menu_delete_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52306, msg = "menu delete error" }, "{}", _e);
                        menu_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await?;
        Ok(())
    }

    /// 创建个性化菜单
    pub async fn add_conditional_menu(
        &self,
        buttons: serde_json::Value,
        matchrule: serde_json::Value,
    ) -> crate::Result<String> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let resp = self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            let buttons = buttons.clone();
            let matchrule = matchrule.clone();
            async move {
                let url = format!("{}/cgi-bin/menu/addconditional?access_token={}", base_url, token);
                let resp: serde_json::Value = client.post(&url)
                    .json(&serde_json::json!({ "button": buttons, "matchrule": matchrule }))
                    .send().await
                    .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 52311, msg = "conditional menu error" }, "{}", _e); menu_conditional_request() })?
                    .json().await
                    .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 52306, msg = "conditional menu error" }, "{}", _e); menu_response() })?;
                check_error(&resp)?;
                Ok(resp)
            }
        }).await?;

        Ok(resp
            .get("menuid")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// 删除个性化菜单
    pub async fn delete_conditional_menu(&self, menuid: &str) -> crate::Result<()> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let menuid = menuid.to_string();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            let menuid = menuid.clone();
            async move {
                let url = format!(
                    "{}/cgi-bin/menu/delconditional?access_token={}",
                    base_url, token
                );
                let resp: serde_json::Value = client
                    .post(&url)
                    .json(&serde_json::json!({ "menuid": menuid }))
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52311, msg = "conditional menu error" }, "{}", _e);
                        menu_conditional_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52306, msg = "conditional menu error" }, "{}", _e);
                        menu_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await?;
        Ok(())
    }

    /// 测试个性化菜单匹配
    pub async fn trymatch_menu(&self, user_id: &str) -> crate::Result<serde_json::Value> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let user_id = user_id.to_string();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            let user_id = user_id.clone();
            async move {
                let url = format!("{}/cgi-bin/menu/trymatch?access_token={}", base_url, token);
                let resp: serde_json::Value = client
                    .post(&url)
                    .json(&serde_json::json!({ "user_id": user_id }))
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52311, msg = "trymatch error" }, "{}", _e);
                        menu_conditional_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52306, msg = "trymatch error" }, "{}", _e);
                        menu_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await
    }

    // ═══════════════════════════════════════════════════════════════
    //  素材管理
    // ═══════════════════════════════════════════════════════════════

    /// 获取永久素材总数
    pub async fn get_material_count(&self) -> crate::Result<MaterialCount> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let resp = self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            async move {
                let url = format!("{}/cgi-bin/material/get_materialcount?access_token={}", base_url, token);
                let resp: serde_json::Value = client.get(&url).send().await
                    .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 52308, msg = "material count error" }, "{}", _e); material_request() })?
                    .json().await
                    .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 52309, msg = "material count error" }, "{}", _e); material_response() })?;
                check_error(&resp)?;
                Ok(resp)
            }
        }).await?;

        serde_json::from_value(resp).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52309, msg = "material count error" }, "{}", _e);
            material_response()
        })
    }

    /// 获取永久素材列表
    pub async fn batchget_material(
        &self,
        material_type: &str,
        offset: i32,
        count: i32,
    ) -> crate::Result<MaterialList> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let material_type = material_type.to_string();
        let resp = self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            let material_type = material_type.clone();
            async move {
                let url = format!("{}/cgi-bin/material/batchget_material?access_token={}", base_url, token);
                let resp: serde_json::Value = client.post(&url)
                    .json(&serde_json::json!({ "type": material_type, "offset": offset, "count": count }))
                    .send().await
                    .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 52308, msg = "material list error" }, "{}", _e); material_request() })?
                    .json().await
                    .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 52309, msg = "material list error" }, "{}", _e); material_response() })?;
                check_error(&resp)?;
                Ok(resp)
            }
        }).await?;

        serde_json::from_value(resp).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52309, msg = "material list error" }, "{}", _e);
            material_response()
        })
    }

    /// 删除永久素材
    pub async fn delete_material(&self, media_id: &str) -> crate::Result<()> {
        let base_url = self.base_url.clone();
        let client = self.client.clone();
        let media_id = media_id.to_string();
        self.do_api(move |token| {
            let base_url = base_url.clone();
            let client = client.clone();
            let media_id = media_id.clone();
            async move {
                let url = format!(
                    "{}/cgi-bin/material/del_material?access_token={}",
                    base_url, token
                );
                let resp: serde_json::Value = client
                    .post(&url)
                    .json(&serde_json::json!({ "media_id": media_id }))
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52308, msg = "material delete error" }, "{}", _e);
                        material_request()
                    })?
                    .json()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 52309, msg = "material delete error" }, "{}", _e);
                        material_response()
                    })?;
                check_error(&resp)?;
                Ok(resp)
            }
        })
        .await?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════
    //  公众号网页授权登录 (原 wx-login-mp)
    // ═══════════════════════════════════════════════════════════════

    /// 拼接完整的 redirect_uri
    pub fn mp_redirect_uri(&self) -> String {
        format!(
            "{}/{}",
            self.redirect_base.trim_end_matches('/'),
            self.callback_path.trim_start_matches('/')
        )
    }

    /// 生成公众号网页授权 URL
    ///
    /// # 参数
    /// - `scope`: 授权作用域，`snsapi_base`（静默）或 `snsapi_userinfo`（弹窗）
    /// - `state`: 可选的 state 参数，用于防止 CSRF 攻击
    pub fn mp_get_authorize_url(&self, scope: &str, state: Option<&str>) -> String {
        let redirect_uri = self.mp_redirect_uri();
        let mut url = format!(
            "https://open.weixin.qq.com/connect/oauth2/authorize?appid={}&redirect_uri={}&response_type=code&scope={}",
            self.app_id, redirect_uri, scope
        );
        if let Some(s) = state {
            url.push_str(&format!("&state={}", s));
        }
        url.push_str("#wechat_redirect");
        url
    }

    /// 通过 code 换取公众号授权 access_token
    pub async fn mp_login(&self, code: &str) -> crate::Result<WxAccessTokenResponse> {
        let mut url = reqwest::Url::parse(&format!("{}/sns/oauth2/access_token", self.base_url))
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52201, msg = "WeChat mp login error" }, "{}", _e);
                mp_url()
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
                tracing::error!({ code = 52202, msg = "WeChat mp login error" }, "{}", _e);
                mp_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52203, msg = "WeChat mp login error" }, "{}", _e);
                mp_response()
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
            tracing::warn!({ code = 42201, msg = "WeChat mp login error" }, "{}", &errmsg);
            return Err(mp_api(&errmsg));
        }

        let token_response: WxAccessTokenResponse =
            serde_json::from_value(response).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52204, msg = "WeChat mp login error" }, "{}", _e);
                mp_parse()
            })?;
        Ok(token_response)
    }

    /// 刷新公众号授权 access_token
    pub async fn mp_refresh_token(
        &self,
        refresh_token: &str,
    ) -> crate::Result<WxAccessTokenResponse> {
        let mut url = reqwest::Url::parse(&format!("{}/sns/oauth2/refresh_token", self.base_url))
            .map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52205, msg = "WeChat mp refresh token error" }, "{}", _e);
            mp_refresh_request()
        })?;
        url.query_pairs_mut()
            .append_pair("appid", &self.app_id)
            .append_pair("grant_type", "refresh_token")
            .append_pair("refresh_token", refresh_token);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52205, msg = "WeChat mp refresh token error" }, "{}", _e);
                mp_refresh_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52206, msg = "WeChat mp refresh token error" }, "{}", _e);
                mp_refresh_response()
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
            tracing::warn!({ code = 42201, msg = "WeChat mp refresh token error" }, "{}", &errmsg);
            return Err(mp_api(&errmsg));
        }

        let token_response: WxAccessTokenResponse =
            serde_json::from_value(response).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52206, msg = "WeChat mp refresh token error" }, "{}", _e);
                mp_refresh_response()
            })?;
        Ok(token_response)
    }

    /// 检验公众号授权 access_token 是否有效
    pub async fn mp_check_token(&self, access_token: &str, openid: &str) -> crate::Result<bool> {
        let mut url =
            reqwest::Url::parse(&format!("{}/sns/auth", self.base_url)).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52207, msg = "WeChat mp check token error" }, "{}", _e);
                mp_check_request()
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
                tracing::error!({ code = 52207, msg = "WeChat mp check token error" }, "{}", _e);
                mp_check_request()
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52208, msg = "WeChat mp check token error" }, "{}", _e);
                mp_check_response()
            })?;

        let errcode = response
            .get("errcode")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1);
        Ok(errcode == 0)
    }

    /// 获取公众号授权用户信息
    pub async fn mp_get_userinfo(
        &self,
        access_token: &str,
        openid: &str,
        lang: Option<&str>,
    ) -> crate::Result<WxUserInfo> {
        let mut url =
            reqwest::Url::parse(&format!("{}/sns/userinfo", self.base_url)).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52209, msg = "WeChat mp userinfo error" }, "{}", _e);
                mp_userinfo_request()
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
                tracing::error!({ code = 52209, msg = "WeChat mp userinfo error" }, "{}", _e);
                mp_userinfo_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52210, msg = "WeChat mp userinfo error" }, "{}", _e);
                mp_userinfo_response()
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
            tracing::warn!({ code = 42201, msg = "WeChat mp userinfo error" }, "{}", &errmsg);
            return Err(mp_api(&errmsg));
        }

        let user_info: WxUserInfo = serde_json::from_value(response).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52210, msg = "WeChat mp userinfo error" }, "{}", _e);
            mp_userinfo_response()
        })?;
        Ok(user_info)
    }

    /// 完整的公众号网页授权登录流程
    pub async fn mp_login_full(&self, code: &str) -> crate::Result<WxMpLoginResult> {
        let token = self.mp_login(code).await?;
        let userinfo = self
            .mp_get_userinfo(&token.access_token, &token.openid, None)
            .await
            .ok();

        Ok(WxMpLoginResult {
            access_token: token.access_token,
            expires_in: token.expires_in,
            refresh_token: token.refresh_token,
            openid: token.openid,
            scope: token.scope,
            unionid: token.unionid,
            userinfo,
        })
    }

    /// 设置公众号网页授权登录回调函数
    pub fn with_mp_callback(
        mut self,
        cb: impl IntoCallback<
            (crate::AppState, WxMpLoginResult, Option<String>),
            crate::Result<WxMpLoginResult>,
        >,
    ) -> Self {
        self.mp_login_callback = Some(cb.into_callback());
        self
    }
}

// ═══════════════════════════════════════════════════════════════
//  素材类型定义
// ═══════════════════════════════════════════════════════════════

/// 永久素材总数
#[derive(Debug, Clone, Serialize, Deserialize, afast::Tag)]
#[tag("永久素材总数")]
pub struct MaterialCount {
    pub voice_count: i64,
    pub video_count: i64,
    pub image_count: i64,
    pub news_count: i64,
}

/// 素材列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialList {
    pub total_count: i64,
    pub item_count: i64,
    pub item: Vec<MaterialItem>,
}

/// 素材项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialItem {
    pub media_id: String,
    pub name: Option<String>,
    pub update_time: Option<i64>,
    pub url: Option<String>,
    #[serde(skip_serializing)]
    pub content: Option<serde_json::Value>,
}

/// 上传临时素材响应
#[derive(Debug, Clone, Serialize, Deserialize, afast::Tag)]
#[tag("上传临时素材响应")]
pub struct TempMediaResponse {
    #[serde(rename = "type")]
    pub media_type: String,
    pub media_id: String,
    pub created_at: i64,
}

/// 上传永久素材响应
#[derive(Debug, Clone, Serialize, Deserialize, afast::Tag)]
#[tag("上传永久素材响应")]
pub struct PermanentMediaResponse {
    pub media_id: String,
    pub url: Option<String>,
}

/// 模板消息构建辅助
pub struct TemplateDataBuilder {
    data: serde_json::Map<String, serde_json::Value>,
}

impl Default for TemplateDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateDataBuilder {
    pub fn new() -> Self {
        Self {
            data: serde_json::Map::new(),
        }
    }

    pub fn value(mut self, key: &str, value: &str) -> Self {
        self.data
            .insert(key.to_string(), serde_json::json!({ "value": value }));
        self
    }

    pub fn value_with_color(mut self, key: &str, value: &str, color: &str) -> Self {
        self.data.insert(
            key.to_string(),
            serde_json::json!({ "value": value, "color": color }),
        );
        self
    }

    pub fn build(self) -> serde_json::Value {
        serde_json::Value::Object(self.data)
    }
}

// ═══════════════════════════════════════════════════════════════
//  公众号网页授权响应类型 (原 wx-login-mp)
// ═══════════════════════════════════════════════════════════════

/// 通过 code 换取的 access_token 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WxAccessTokenResponse {
    pub access_token: String,
    pub expires_in: i32,
    pub refresh_token: String,
    pub openid: String,
    pub scope: String,
    pub unionid: Option<String>,
}

/// 公众号授权用户信息
#[derive(Debug, Clone, Serialize, Deserialize, afast::Tag)]
#[tag("公众号授权用户信息")]
pub struct WxUserInfo {
    pub openid: String,
    pub nickname: Option<String>,
    pub sex: Option<i32>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub headimgurl: Option<String>,
    pub privilege: Option<Vec<String>>,
    pub unionid: Option<String>,
}

/// 公众号网页授权登录结果
#[derive(Debug, Clone, Serialize, Deserialize, afast::Tag)]
#[tag("公众号网页授权登录结果")]
pub struct WxMpLoginResult {
    pub access_token: String,
    pub expires_in: i32,
    pub refresh_token: String,
    pub openid: String,
    pub scope: String,
    pub unionid: Option<String>,
    pub userinfo: Option<WxUserInfo>,
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 微信公众号配置扩展
pub trait AFasterWxOfficialExt {
    /// 链式配置微信公众号
    fn with_wx_official(self, f: impl FnOnce(WxOfficial) -> WxOfficial) -> Self;
}

impl AFasterWxOfficialExt for crate::AFaster {
    fn with_wx_official(mut self, f: impl FnOnce(WxOfficial) -> WxOfficial) -> Self {
        self.state.wx_official = f(self.state.wx_official);
        self
    }
}

impl WxOfficial {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "wx_official")?;
        instance.client = super::default_http_client();
        Ok(instance)
    }
}
