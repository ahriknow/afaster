use serde::{Deserialize, Serialize};

use super::{Notification, PushTarget};

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

#[derive(Clone, Deserialize)]
pub struct JPush {
    /// 极光应用 AppKey
    pub app_key: String,
    /// 极光应用 Master Secret
    pub master_secret: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub apns_production: bool,
    #[serde(skip)]
    pub client: reqwest::Client,
}

fn default_base_url() -> String {
    "https://api.jpush.cn".to_string()
}

// ═══════════════════════════════════════════════════════════════
//  推送请求结构
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Serialize)]
struct JPushRequest {
    platform: serde_json::Value,
    audience: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cid: Option<String>,
}

#[derive(Debug, Serialize)]
struct JPushRequestOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    time_to_live: Option<i64>,
    apns_production: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    big_push_duration: Option<i32>,
}

/// 极光推送响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JPushResult {
    pub sendno: Option<String>,
    pub msg_id: Option<String>,
    #[serde(flatten)]
    pub error: Option<serde_json::Value>,
}

impl JPushResult {
    pub fn is_ok(&self) -> bool {
        self.msg_id.is_some()
    }

    pub fn error_code(&self) -> Option<i64> {
        self.error
            .as_ref()
            .and_then(|e| e.get("error_code").and_then(|v| v.as_i64()))
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error
            .as_ref()
            .and_then(|e| e.get("error_message").and_then(|v| v.as_str()))
    }
}

// ═══════════════════════════════════════════════════════════════
//  推送设置
// ═══════════════════════════════════════════════════════════════

/// 极光推送设置
#[derive(Debug, Clone, Default)]
pub struct JPushOptions {
    /// 离线消息保留时长（秒），默认 86400（1天），最长 10 天（VIP）
    pub time_to_live: Option<i64>,
    /// APNs 是否生产环境（默认 true）
    pub apns_production: Option<bool>,
    /// 定速推送时长（分钟），最大 1400
    pub big_push_duration: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════
//  实现
// ═══════════════════════════════════════════════════════════════

impl JPush {
    /// 获取 Basic Auth 头
    fn auth_header(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.app_key, self.master_secret);
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
        )
    }

    /// 发送通知消息
    pub async fn send(
        &self,
        target: &PushTarget,
        notification: &Notification,
        options: Option<&JPushOptions>,
    ) -> crate::Result<JPushResult> {
        let audience = match target {
            PushTarget::Cid(cid) => serde_json::json!({ "registration_id": [cid] }),
            PushTarget::CidList(cids) => serde_json::json!({ "registration_id": cids }),
            PushTarget::Alias(alias) => serde_json::json!({ "alias": [alias] }),
            PushTarget::AliasList(aliases) => serde_json::json!({ "alias": aliases }),
            PushTarget::Tag(tag) => serde_json::json!({ "tag": [tag] }),
            PushTarget::All => serde_json::json!("all"),
        };

        let android_alert = notification.body.clone();
        let ios_alert = notification.body.clone();

        let mut android = serde_json::json!({
            "alert": android_alert,
            "title": notification.title,
        });
        if let Some(ref click_type) = notification.click_type
            && click_type == "url"
            && let Some(ref url) = notification.url
        {
            android["intent"] = serde_json::json!({
                "url": url
            });
        }
        if let Some(ref extras) = notification.logo_url {
            android["large_icon"] = serde_json::json!(extras);
        }

        let ios = serde_json::json!({
            "alert": ios_alert,
            "sound": "default",
        });

        let notif = serde_json::json!({
            "alert": notification.body,
            "android": android,
            "ios": ios,
        });

        let opts = options.map(|o| {
            serde_json::json!({
                "time_to_live": o.time_to_live,
                "apns_production": o.apns_production.unwrap_or(self.apns_production),
                "big_push_duration": o.big_push_duration,
            })
        });

        let body = JPushRequest {
            platform: serde_json::json!("all"),
            audience,
            notification: Some(notif),
            message: None,
            options: opts,
            cid: None,
        };

        self.do_push(&body).await
    }

    /// 发送透传消息（自定义消息）
    pub async fn send_message(
        &self,
        target: &PushTarget,
        content: &str,
        content_type: Option<&str>,
        extras: Option<serde_json::Value>,
        options: Option<&JPushOptions>,
    ) -> crate::Result<JPushResult> {
        let audience = match target {
            PushTarget::Cid(cid) => serde_json::json!({ "registration_id": [cid] }),
            PushTarget::CidList(cids) => serde_json::json!({ "registration_id": cids }),
            PushTarget::Alias(alias) => serde_json::json!({ "alias": [alias] }),
            PushTarget::AliasList(aliases) => serde_json::json!({ "alias": aliases }),
            PushTarget::Tag(tag) => serde_json::json!({ "tag": [tag] }),
            PushTarget::All => serde_json::json!("all"),
        };

        let mut msg = serde_json::json!({
            "msg_content": content,
        });
        if let Some(ct) = content_type {
            msg["content_type"] = serde_json::json!(ct);
        }
        if let Some(ext) = extras {
            msg["extras"] = ext;
        }

        let opts = options.map(|o| {
            serde_json::json!({
                "time_to_live": o.time_to_live,
                "apns_production": o.apns_production.unwrap_or(self.apns_production),
                "big_push_duration": o.big_push_duration,
            })
        });

        let body = JPushRequest {
            platform: serde_json::json!("all"),
            audience,
            notification: None,
            message: Some(msg),
            options: opts,
            cid: None,
        };

        self.do_push(&body).await
    }

    /// 同时发送通知 + 透传消息
    pub async fn send_notification_and_message(
        &self,
        target: &PushTarget,
        notification: &Notification,
        content: &str,
        options: Option<&JPushOptions>,
    ) -> crate::Result<JPushResult> {
        let audience = match target {
            PushTarget::Cid(cid) => serde_json::json!({ "registration_id": [cid] }),
            PushTarget::CidList(cids) => serde_json::json!({ "registration_id": cids }),
            PushTarget::Alias(alias) => serde_json::json!({ "alias": [alias] }),
            PushTarget::AliasList(aliases) => serde_json::json!({ "alias": aliases }),
            PushTarget::Tag(tag) => serde_json::json!({ "tag": [tag] }),
            PushTarget::All => serde_json::json!("all"),
        };

        let notif = serde_json::json!({
            "alert": notification.body,
            "android": {
                "alert": notification.body,
                "title": notification.title,
            },
            "ios": {
                "alert": notification.body,
                "sound": "default",
            },
        });

        let msg = serde_json::json!({
            "msg_content": content,
        });

        let opts = options.map(|o| {
            serde_json::json!({
                "time_to_live": o.time_to_live,
                "apns_production": o.apns_production.unwrap_or(self.apns_production),
                "big_push_duration": o.big_push_duration,
            })
        });

        let body = JPushRequest {
            platform: serde_json::json!("all"),
            audience,
            notification: Some(notif),
            message: Some(msg),
            options: opts,
            cid: None,
        };

        self.do_push(&body).await
    }

    /// 执行推送请求
    async fn do_push(&self, body: &JPushRequest) -> crate::Result<JPushResult> {
        let url = format!("{}/v3/push", self.base_url);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52601, msg = "JPush push error" }, "{}", _e);
                crate::Error::custom(52601, "JPush push request failed")
            })?;

        let status = resp.status();
        let resp_text = resp.text().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52602, msg = "JPush push error" }, "{}", _e);
            crate::Error::custom(52602, "JPush push response read failed")
        })?;

        if status.is_success() {
            let result: JPushResult = serde_json::from_str(&resp_text).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52603, msg = "JPush push error" }, "{}", _e);
                crate::Error::custom(52603, "JPush push response parse failed")
            })?;
            Ok(result)
        } else {
            let result: JPushResult = serde_json::from_str(&resp_text).unwrap_or(JPushResult {
                sendno: None,
                msg_id: None,
                error: Some(serde_json::json!({ "error_code": status.as_u16(), "error_message": resp_text })),
            });
            #[cfg(feature = "log")]
            tracing::warn!(
                { code = 42601, msg = "JPush push error" },
                "status={}, body={}", status, resp_text
            );
            Ok(result)
        }
    }
}
