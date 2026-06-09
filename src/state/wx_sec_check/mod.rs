#[allow(dead_code)]
mod err;
use err::*;

pub mod callback;
pub use callback::{MediaCheckCallbackFn, WxSecCheckNotifyResult};

use crate::state::callbacks::IntoCallback;
pub(crate) mod notify;
use notify::WxNotifyBody;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

fn default_callback_path() -> String {
    "wx/sec-check/notify".to_string()
}

// ═══════════════════════════════════════════════════════════════
//  配置结构体
// ═══════════════════════════════════════════════════════════════

fn default_base_url() -> String {
    "https://api.weixin.qq.com".to_string()
}

#[derive(Clone, Deserialize)]
pub struct WxSecCheck {
    pub app_id: String,
    pub app_secret: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,

    // ── 回调配置 ──
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
    #[serde(default)]
    pub verify_token: String,
    #[serde(default)]
    pub msg_secret: String,

    // ── 回调函数 ──
    #[serde(skip)]
    pub(crate) media_check_callback: Option<MediaCheckCallbackFn>,

    #[serde(skip)]
    pub client: Client,
    #[serde(skip)]
    token: Arc<Mutex<Option<CachedToken>>>,
}

struct CachedToken {
    token: String,
    expires_at: Instant,
}

// ═══════════════════════════════════════════════════════════════
//  公开枚举类型
// ═══════════════════════════════════════════════════════════════

/// 场景枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Scene {
    /// 资料
    Profile = 1,
    /// 评论
    Comment = 2,
    /// 论坛
    Forum = 3,
    /// 社交日志
    Social = 4,
}

/// 建议枚举
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Suggest {
    Pass,
    Review,
    Risky,
}

/// 标签枚举
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Label {
    #[serde(rename = "100")]
    Normal,
    #[serde(rename = "10001")]
    Ad,
    #[serde(rename = "20001")]
    Politics,
    #[serde(rename = "20002")]
    Porn,
    #[serde(rename = "20003")]
    Abuse,
    #[serde(rename = "20006")]
    Illegal,
    #[serde(rename = "20008")]
    Fraud,
    #[serde(rename = "20012")]
    Vulgar,
    #[serde(rename = "20013")]
    Copyright,
    #[serde(rename = "21000")]
    Other,
}

/// 媒体类型枚举
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum MediaType {
    /// 音频 (mp3, aac, ac3, wma, flac, vorbis, opus, wav)
    Audio = 1,
    /// 图片 (jpg, jpeg, png, bmp, gif)
    Image = 2,
}

// ═══════════════════════════════════════════════════════════════
//  文本内容安全 — 请求 / 响应
// ═══════════════════════════════════════════════════════════════

/// 文本内容安全检测请求
#[derive(Debug, Serialize)]
pub struct MsgSecCheckRequest {
    pub content: String,
    pub version: u8,
    pub scene: u8,
    pub openid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl MsgSecCheckRequest {
    /// 创建文本检测请求（version 固定为 2）
    pub fn new(content: impl Into<String>, scene: Scene, openid: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            version: 2,
            scene: scene as u8,
            openid: openid.into(),
            title: None,
            nickname: None,
            signature: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn nickname(mut self, nickname: impl Into<String>) -> Self {
        self.nickname = Some(nickname.into());
        self
    }

    pub fn signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = Some(signature.into());
        self
    }
}

/// 文本内容安全检测响应
#[derive(Debug, Deserialize)]
pub struct MsgSecCheckResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub trace_id: Option<String>,
    pub result: Option<SecCheckResult>,
    pub detail: Option<Vec<SecCheckDetail>>,
}

/// 综合结果
#[derive(Debug, Deserialize, afast::Tag)]
#[tag("内容安全综合结果")]
pub struct SecCheckResult {
    pub suggest: String,
    pub label: i32,
}

/// 详细检测结果
#[derive(Debug, Deserialize, afast::Tag)]
#[tag("内容安全详细检测结果")]
pub struct SecCheckDetail {
    pub strategy: String,
    pub errcode: i32,
    pub suggest: String,
    pub label: i32,
    #[serde(default)]
    pub prob: Option<i32>,
    #[serde(default)]
    pub level: Option<i32>,
    #[serde(default)]
    pub keyword: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  媒体内容安全 — 请求 / 响应
// ═══════════════════════════════════════════════════════════════

/// 异步媒体检测请求
#[derive(Debug, Serialize)]
pub struct MediaCheckRequest {
    pub media_url: String,
    pub media_type: u8,
    pub version: u8,
    pub scene: u8,
    pub openid: String,
}

impl MediaCheckRequest {
    /// 创建媒体检测请求（version 固定为 2）
    pub fn new(
        media_url: impl Into<String>,
        media_type: MediaType,
        scene: Scene,
        openid: impl Into<String>,
    ) -> Self {
        Self {
            media_url: media_url.into(),
            media_type: media_type as u8,
            version: 2,
            scene: scene as u8,
            openid: openid.into(),
        }
    }
}

/// 异步媒体检测响应（发起请求时同步返回）
#[derive(Debug, Deserialize)]
pub struct MediaCheckResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub trace_id: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  媒体检测异步推送结果（微信服务器主动推送）
// ═══════════════════════════════════════════════════════════════

/// 媒体检测异步推送通知
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MediaCheckNotify {
    pub appid: Option<String>,
    pub trace_id: Option<String>,
    pub version: Option<i32>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub result: Option<SecCheckResult>,
    pub detail: Option<Vec<SecCheckDetail>>,
}

impl afast::Structure for MediaCheckNotify {
    fn structure() -> &'static afast::TagMeta {
        use afast::handler::{FieldMeta, TagKind, TagMeta, no_structure};
        static META: TagMeta = TagMeta {
            name: "MediaCheckNotify",
            desc: "媒体检测异步推送通知",
            kind: TagKind::Struct(&[
                FieldMeta {
                    name: "appid",
                    ty: "Option<String>",
                    desc: "",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "trace_id",
                    ty: "Option<String>",
                    desc: "",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "version",
                    ty: "Option<i32>",
                    desc: "",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "errcode",
                    ty: "Option<i32>",
                    desc: "",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "errmsg",
                    ty: "Option<String>",
                    desc: "",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "result",
                    ty: "Option<SecCheckResult>",
                    desc: "综合结果",
                    structure: Some(SecCheckResult::structure),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "detail",
                    ty: "Option<Vec<SecCheckDetail>>",
                    desc: "详细检测结果",
                    structure: Some(SecCheckDetail::structure),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
            ]),
        };
        &META
    }
}

// ═══════════════════════════════════════════════════════════════
//  access_token 响应（内部）
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AccessTokenResponse {
    access_token: Option<String>,
    expires_in: Option<u64>,
    errcode: Option<i32>,
    errmsg: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  核心实现
// ═══════════════════════════════════════════════════════════════

impl WxSecCheck {
    /// 获取 access_token（内部方法，自动缓存和刷新）
    async fn get_access_token(&self) -> crate::Result<String> {
        // 检查缓存
        {
            let guard = self.token.lock().await;
            if let Some(cached) = guard.as_ref()
                && Instant::now() < cached.expires_at
            {
                return Ok(cached.token.clone());
            }
        }

        // 请求新 token
        let mut url =
            reqwest::Url::parse(&format!("{}/cgi-bin/token", self.base_url)).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50705, msg = "Failed to get access_token" }, "{}", _e);
                token()
            })?;
        url.query_pairs_mut()
            .append_pair("grant_type", "client_credential")
            .append_pair("appid", &self.app_id)
            .append_pair("secret", &self.app_secret);

        let resp = self.client.get(url).send().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50705, msg = "Failed to get access_token" }, "{}", _e);
            token()
        })?;

        let body: serde_json::Value = resp.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50706, msg = "Access_token response parse failed" }, "{}", _e);
            token_parse()
        })?;

        // 检查错误
        if let Some(errcode) = body.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = body
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            #[cfg(feature = "log")]
            tracing::warn!({ code = 40701, msg = "Failed to get access_token" }, "{}", errmsg);
            return Err(api(errmsg));
        }

        let at_resp: AccessTokenResponse = serde_json::from_value(body).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50706, msg = "Access_token deserialize failed" }, "{}", _e);
            token_parse()
        })?;

        let token = at_resp.access_token.ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50705, msg = "Access_token is empty" }, "access_token is empty");
            token()
        })?;

        let expires_in = at_resp.expires_in.unwrap_or(7200);
        // 提前 5 分钟刷新
        let expires_at = Instant::now() + Duration::from_secs(expires_in.saturating_sub(300));

        // 写入缓存
        {
            let mut guard = self.token.lock().await;
            *guard = Some(CachedToken {
                token: token.clone(),
                expires_at,
            });
        }

        Ok(token)
    }

    /// 清除 token 缓存（用于被动刷新）
    async fn clear_token(&self) {
        self.token.lock().await.take();
    }

    /// 判断是否为 token 过期错误
    fn is_token_error(body: &serde_json::Value) -> bool {
        matches!(
            body.get("errcode").and_then(|v| v.as_i64()),
            Some(40001 | 42001)
        )
    }

    /// 文本内容安全检测
    ///
    /// # 参数
    /// - `request`: MsgSecCheckRequest
    ///
    /// # 返回
    /// - `crate::Result<MsgSecCheckResponse>`
    pub async fn msg_sec_check(
        &self,
        request: &MsgSecCheckRequest,
    ) -> crate::Result<MsgSecCheckResponse> {
        // 校验 content 长度
        if request.content.is_empty() || request.content.len() > 2500 * 4 {
            // UTF-8 中一个中文字符最多 4 字节，2500 字 ≈ 10000 字节
            // 但实际判断应该用字符数
            let char_count = request.content.chars().count();
            if char_count == 0 || char_count > 2500 {
                #[cfg(feature = "log")]
                tracing::warn!(
                    { code = 40702, msg = "Invalid content" },
                    "content 字符数: {}",
                    char_count
                );
                return Err(content_invalid());
            }
        }

        let access_token = self.get_access_token().await?;
        let base_url = self.base_url.clone();

        let mut url =
            reqwest::Url::parse(&format!("{}/wxa/msg_sec_check", base_url)).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50701, msg = "URL build failed" }, "{}", _e);
                url()
            })?;
        url.query_pairs_mut()
            .append_pair("access_token", &access_token);

        let resp = self
            .client
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50702, msg = "Request failed" }, "{}", _e);
                err::request()
            })?;

        let body: serde_json::Value = resp.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50703, msg = "Response JSON parse failed" }, "{}", _e);
            err::response()
        })?;

        // 检查微信 API 错误
        if let Some(errcode) = body.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            // 被动刷新 token
            if Self::is_token_error(&body) {
                #[cfg(feature = "log")]
                tracing::warn!("wx_sec_check token expired, refreshing...");
                self.clear_token().await;
                let new_token = self.get_access_token().await?;
                let mut url2 = reqwest::Url::parse(&format!("{}/wxa/msg_sec_check", base_url))
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 50701, msg = "URL build failed" }, "{}", _e);
                        err::url()
                    })?;
                url2.query_pairs_mut()
                    .append_pair("access_token", &new_token);
                let resp2 = self
                    .client
                    .post(url2)
                    .json(request)
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 50702, msg = "Request failed" }, "{}", _e);
                        err::request()
                    })?;
                let body2: serde_json::Value = resp2.json().await.map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!({ code = 50703, msg = "Response JSON parse failed" }, "{}", _e);
                    err::response()
                })?;
                let result: MsgSecCheckResponse = serde_json::from_value(body2)
                        .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 50704, msg = "Response deserialize failed" }, "{}", _e); parse() })?;
                return Ok(result);
            }
            let errmsg = body
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 40701, msg = "WeChat API error" }, "{}", &errmsg);
            return Err(api(&errmsg));
        }

        let result: MsgSecCheckResponse = serde_json::from_value(body).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50704, msg = "Response deserialize failed" }, "{}", _e);
            parse()
        })?;

        Ok(result)
    }

    /// 异步媒体内容安全检测
    ///
    /// # 参数
    /// - `request`: MediaCheckRequest
    ///
    /// # 返回
    /// - `crate::Result<MediaCheckResponse>`
    pub async fn media_check_async(
        &self,
        request: &MediaCheckRequest,
    ) -> crate::Result<MediaCheckResponse> {
        let access_token = self.get_access_token().await?;
        let base_url = self.base_url.clone();

        let mut url =
            reqwest::Url::parse(&format!("{}/wxa/media_check_async", base_url)).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50701, msg = "URL build failed" }, "{}", _e);
                url()
            })?;
        url.query_pairs_mut()
            .append_pair("access_token", &access_token);

        let resp = self
            .client
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50702, msg = "Request failed" }, "{}", _e);
                err::request()
            })?;

        let body: serde_json::Value = resp.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50703, msg = "Response JSON parse failed" }, "{}", _e);
            err::response()
        })?;

        // 检查微信 API 错误
        if let Some(errcode) = body.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            // 被动刷新 token
            if Self::is_token_error(&body) {
                #[cfg(feature = "log")]
                tracing::warn!("wx_sec_check token expired, refreshing...");
                self.clear_token().await;
                let new_token = self.get_access_token().await?;
                let mut url2 = reqwest::Url::parse(&format!("{}/wxa/media_check_async", base_url))
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 50701, msg = "URL build failed" }, "{}", _e);
                        err::url()
                    })?;
                url2.query_pairs_mut()
                    .append_pair("access_token", &new_token);
                let resp2 = self
                    .client
                    .post(url2)
                    .json(request)
                    .send()
                    .await
                    .map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!({ code = 50702, msg = "Request failed" }, "{}", _e);
                        err::request()
                    })?;
                let body2: serde_json::Value = resp2.json().await.map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!({ code = 50703, msg = "Response JSON parse failed" }, "{}", _e);
                    err::response()
                })?;
                let result: MediaCheckResponse = serde_json::from_value(body2)
                        .map_err(|_e| { #[cfg(feature = "log")] tracing::error!({ code = 50704, msg = "Response deserialize failed" }, "{}", _e); parse() })?;
                return Ok(result);
            }
            let errmsg = body
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!({ code = 40701, msg = "WeChat API error" }, "{}", &errmsg);
            return Err(api(&errmsg));
        }

        let result: MediaCheckResponse = serde_json::from_value(body).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50704, msg = "Response deserialize failed" }, "{}", _e);
            parse()
        })?;

        Ok(result)
    }

    // ═══════════════════════════════════════════════════════════════
    //  回调注册
    // ═══════════════════════════════════════════════════════════════

    /// 设置媒体检测回调
    pub fn with_media_check_callback(
        mut self,
        cb: impl IntoCallback<
            (crate::AppState, callback::MediaCheckNotify),
            crate::Result<WxSecCheckNotifyResult>,
        >,
    ) -> Self {
        self.media_check_callback = Some(cb.into_callback());
        self
    }

    // ═══════════════════════════════════════════════════════════════
    //  回调分发（供 afast handler 调用）
    // ═══════════════════════════════════════════════════════════════

    /// 处理微信内容安全回调通知
    pub async fn handle_notify(
        &self,
        state: crate::AppState,
        mut body: WxNotifyBody,
    ) -> crate::Result<afast::Text> {
        if let Some(encrypt_b64) = &body.encrypt {
            if self.msg_secret.is_empty() {
                return Err(err::decrypt_failed("msg_secret not configured"));
            }
            let json_str = notify::decrypt_wx_message(encrypt_b64, &self.msg_secret)?;
            body = serde_json::from_str(&json_str)
                .map_err(|_e| crate::Error::custom(50801, "Decrypted message parse failed"))?;
        }

        let body_json = serde_json::to_value(&body)
            .map_err(|_e| crate::Error::custom(50801, "Notification body serialize failed"))?;

        match body.event.as_deref() {
            Some("wxa_media_check") => {
                let notify: callback::MediaCheckNotify = serde_json::from_value(body_json)
                    .map_err(|_e| {
                        crate::Error::custom(50806, "Media detection notification parse failed")
                    })?;
                if let Some(cb) = self.media_check_callback.clone() {
                    let result = cb((state.clone(), notify)).await?;
                    Ok(afast::Text(
                        serde_json::to_string(&result).unwrap_or_default(),
                    ))
                } else {
                    #[cfg(feature = "log")]
                    tracing::debug!("WxSecCheck media_check callback not registered");
                    Ok(afast::Text(
                        r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                    ))
                }
            }
            _ => {
                #[cfg(feature = "log")]
                tracing::warn!("WxSecCheck unknown event: {:?}", body.event);
                Ok(afast::Text(
                    r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                ))
            }
        }
    }
}
