use serde::{Deserialize, Serialize};

use super::{Notification, PushTarget};

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

#[derive(Clone, Deserialize)]
pub struct XiaomiPush {
    /// 应用包名
    pub package_name: String,
    /// APP_SECRET（用于 Authorization 头）
    pub app_secret: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(skip)]
    pub client: reqwest::Client,
}

fn default_base_url() -> String {
    "https://api.xmpush.xiaomi.com".to_string()
}

// ═══════════════════════════════════════════════════════════════
//  推送设置
// ═══════════════════════════════════════════════════════════════

/// 小米推送设置
#[derive(Debug, Clone, Default)]
pub struct XiaomiPushOptions {
    /// 消息有效期（毫秒），默认 1 天
    pub time_to_live: Option<i64>,
    /// 定时发送（毫秒时间戳，7天内）
    pub time_to_send: Option<i64>,
    /// 通知栏 ID（相同 ID 会覆盖）
    pub notify_id: Option<i32>,
    /// 通知类型：1=声音, 2=震动, 3=声音+震动, 4=静默
    pub notify_type: Option<i32>,
    /// 通知渠道 ID（Android 8.0+）
    pub channel_id: Option<String>,
    /// 点击行为：1=打开首页, 2=打开 Activity, 3=打开网页
    pub notify_effect: Option<i32>,
    /// 打开网页地址（notify_effect=3 时）
    pub web_uri: Option<String>,
    /// 打开 Activity（notify_effect=2 时）
    pub intent_uri: Option<String>,
    /// 自定义铃声 URI
    pub sound_uri: Option<String>,
    /// 前台是否弹出通知
    pub notify_foreground: Option<bool>,
    /// 平滑推送速度（条/秒，最低 3000）
    pub flow_control: Option<i32>,
    /// 消息去重 key
    pub jobkey: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  推送响应
// ═══════════════════════════════════════════════════════════════

/// 小米推送响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XiaomiPushResult {
    pub result: Option<String>,
    pub code: Option<i64>,
    pub description: Option<String>,
    pub info: Option<String>,
    pub data: Option<serde_json::Value>,
    pub reason: Option<String>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl XiaomiPushResult {
    pub fn is_ok(&self) -> bool {
        self.result.as_deref() == Some("ok") || self.code == Some(0)
    }

    pub fn error_message(&self) -> &str {
        self.reason
            .as_deref()
            .or(self.description.as_deref())
            .unwrap_or("unknown error")
    }

    pub fn message_id(&self) -> Option<&str> {
        self.data
            .as_ref()
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
    }
}

// ═══════════════════════════════════════════════════════════════
//  实现
// ═══════════════════════════════════════════════════════════════

impl XiaomiPush {
    /// 构建 extra.* 参数
    fn build_extra(options: &XiaomiPushOptions) -> Vec<(String, String)> {
        let mut params = Vec::new();
        if let Some(ref channel_id) = options.channel_id {
            params.push(("extra.channel_id".to_string(), channel_id.clone()));
        }
        if let Some(notify_effect) = options.notify_effect {
            params.push(("extra.notify_effect".to_string(), notify_effect.to_string()));
        }
        if let Some(ref web_uri) = options.web_uri {
            params.push(("extra.web_uri".to_string(), web_uri.clone()));
        }
        if let Some(ref intent_uri) = options.intent_uri {
            params.push(("extra.intent_uri".to_string(), intent_uri.clone()));
        }
        if let Some(ref sound_uri) = options.sound_uri {
            params.push(("extra.sound_uri".to_string(), sound_uri.clone()));
        }
        if let Some(notify_foreground) = options.notify_foreground {
            params.push((
                "extra.notify_foreground".to_string(),
                if notify_foreground {
                    "1".to_string()
                } else {
                    "0".to_string()
                },
            ));
        }
        if let Some(flow_control) = options.flow_control {
            params.push(("extra.flow_control".to_string(), flow_control.to_string()));
        }
        if let Some(ref jobkey) = options.jobkey {
            params.push(("extra.jobkey".to_string(), jobkey.clone()));
        }
        params
    }

    /// 发送通知消息
    pub async fn send(
        &self,
        target: &PushTarget,
        notification: &Notification,
        options: Option<&XiaomiPushOptions>,
    ) -> crate::Result<XiaomiPushResult> {
        let opts = options.cloned().unwrap_or_default();

        let mut params = vec![
            (
                "restricted_package_name".to_string(),
                self.package_name.clone(),
            ),
            ("title".to_string(), notification.title.clone()),
            ("description".to_string(), notification.body.clone()),
            ("payload".to_string(), notification.body.clone()),
            (
                "notify_type".to_string(),
                opts.notify_type.unwrap_or(3).to_string(),
            ),
        ];

        if let Some(ttl) = opts.time_to_live {
            params.push(("time_to_live".to_string(), ttl.to_string()));
        }
        if let Some(tts) = opts.time_to_send {
            params.push(("time_to_send".to_string(), tts.to_string()));
        }
        if let Some(nid) = opts.notify_id {
            params.push(("notify_id".to_string(), nid.to_string()));
        }

        // extra 参数
        params.extend(Self::build_extra(&opts));

        // 目标参数
        let path = match target {
            PushTarget::Cid(cid) => {
                params.push(("registration_id".to_string(), cid.clone()));
                "/v3/message/regid"
            }
            PushTarget::CidList(cids) => {
                params.push(("registration_id".to_string(), cids.join(",")));
                "/v3/message/regid"
            }
            PushTarget::Alias(alias) => {
                params.push(("alias".to_string(), alias.clone()));
                "/v3/message/alias"
            }
            PushTarget::AliasList(aliases) => {
                params.push(("alias".to_string(), aliases.join(",")));
                "/v3/message/alias"
            }
            PushTarget::Tag(tag) => {
                params.push(("topic".to_string(), tag.clone()));
                "/v3/message/topic"
            }
            PushTarget::All => "/v3/message/all",
        };

        self.do_push(path, &params).await
    }

    /// 发送透传消息
    pub async fn send_transmission(
        &self,
        target: &PushTarget,
        content: &str,
        options: Option<&XiaomiPushOptions>,
    ) -> crate::Result<XiaomiPushResult> {
        let opts = options.cloned().unwrap_or_default();

        let mut params = vec![
            (
                "restricted_package_name".to_string(),
                self.package_name.clone(),
            ),
            ("payload".to_string(), content.to_string()),
            ("pass_through".to_string(), "1".to_string()),
            ("notify_type".to_string(), "4".to_string()),
        ];

        if let Some(ttl) = opts.time_to_live {
            params.push(("time_to_live".to_string(), ttl.to_string()));
        }

        // extra 参数
        params.extend(Self::build_extra(&opts));

        // 目标参数
        let path = match target {
            PushTarget::Cid(cid) => {
                params.push(("registration_id".to_string(), cid.clone()));
                "/v3/message/regid"
            }
            PushTarget::CidList(cids) => {
                params.push(("registration_id".to_string(), cids.join(",")));
                "/v3/message/regid"
            }
            PushTarget::Alias(alias) => {
                params.push(("alias".to_string(), alias.clone()));
                "/v3/message/alias"
            }
            PushTarget::AliasList(aliases) => {
                params.push(("alias".to_string(), aliases.join(",")));
                "/v3/message/alias"
            }
            PushTarget::Tag(tag) => {
                params.push(("topic".to_string(), tag.clone()));
                "/v3/message/topic"
            }
            PushTarget::All => "/v3/message/all",
        };

        self.do_push(path, &params).await
    }

    /// 执行推送请求
    async fn do_push(
        &self,
        path: &str,
        params: &[(String, String)],
    ) -> crate::Result<XiaomiPushResult> {
        let url = format!("{}{}", self.base_url, path);

        let resp: XiaomiPushResult = self
            .client
            .post(&url)
            .header("Authorization", format!("key={}", self.app_secret))
            .form(params)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52701, msg = "Xiaomi push error" }, "{}", _e);
                crate::Error::custom(52701, "Xiaomi push request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52702, msg = "Xiaomi push error" }, "{}", _e);
                crate::Error::custom(52702, "Xiaomi push response parse failed")
            })?;

        if !resp.is_ok() {
            #[cfg(feature = "log")]
            tracing::warn!(
                { code = 42701, msg = "Xiaomi push error" },
                "result={}, code={}, desc={}", resp.result.as_deref().unwrap_or(""), resp.code.unwrap_or(-1), resp.error_message()
            );
        }

        Ok(resp)
    }
}
