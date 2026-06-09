use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{Notification, PushResult, PushSettings, PushTarget};

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

#[derive(Clone, Deserialize)]
pub struct GeTui {
    pub app_id: String,
    pub app_key: String,
    pub master_secret: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(skip)]
    pub client: reqwest::Client,
    #[serde(skip)]
    token_cache: Arc<Mutex<Option<CachedToken>>>,
}

fn default_base_url() -> String {
    "https://restapi.getui.com".to_string()
}

#[derive(Clone)]
struct CachedToken {
    token: String,
    expires_at: std::time::Instant,
}

// ═══════════════════════════════════════════════════════════════
//  Token 响应
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct TokenResponse {
    code: i32,
    msg: String,
    data: Option<TokenData>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct TokenData {
    token: String,
    /// 过期时间（毫秒时间戳）
    expire_time: String,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

// ═══════════════════════════════════════════════════════════════
//  推送消息结构
// ═══════════════════════════════════════════════════════════════

/// 单推请求体
#[derive(Debug, Clone, Serialize)]
struct PushRequestBody {
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<PushSettings>,
    audience: serde_json::Value,
    push_message: PushMessageBody,
}

#[derive(Debug, Clone, Serialize)]
struct PushMessageBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    notification: Option<Notification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transmission: Option<String>,
}

/// 批量单推请求体
#[derive(Debug, Serialize)]
struct BatchPushRequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    is_async: Option<bool>,
    msg_list: Vec<PushRequestBody>,
}

/// 创建消息请求体（toList 第一步）
#[derive(Debug, Serialize)]
struct CreateMessageBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    settings: Option<PushSettings>,
    push_message: PushMessageBody,
}

/// 创建消息响应
#[derive(Debug, Deserialize)]
struct CreateMessageResponse {
    code: i32,
    msg: String,
    data: Option<CreateMessageData>,
}

#[derive(Debug, Deserialize)]
struct CreateMessageData {
    taskid: String,
}

/// 批量推请求体（toList 第二步）
#[derive(Debug, Serialize)]
struct ListPushBody {
    audience: serde_json::Value,
    taskid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_async: Option<bool>,
}

// ═══════════════════════════════════════════════════════════════
//  实现
// ═══════════════════════════════════════════════════════════════

impl GeTui {
    fn base_url(&self) -> String {
        format!("{}/v2/{}", self.base_url, self.app_id)
    }

    /// 获取 token（自动缓存，过期前 1 小时刷新）
    pub async fn get_token(&self) -> crate::Result<String> {
        {
            let cache = self.token_cache.lock().await;
            if let Some(ref cached) = *cache
                && cached.expires_at > std::time::Instant::now()
            {
                return Ok(cached.token.clone());
            }
        }
        self.refresh_token().await
    }

    /// 刷新 token
    pub async fn refresh_token(&self) -> crate::Result<String> {
        use sha2::{Digest, Sha256};

        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
        let sign_input = format!("{}{}{}", self.app_key, timestamp, self.master_secret);
        let hash = Sha256::digest(sign_input.as_bytes());
        let sign: String = hash.iter().map(|b| format!("{:02x}", b)).collect();

        let body = serde_json::json!({
            "sign": sign,
            "timestamp": timestamp,
            "appkey": self.app_key,
        });

        let url = format!("{}/auth", self.base_url());

        let resp: TokenResponse = self
            .client
            .post(&url)
            .header("Content-Type", "application/json;charset=utf-8")
            .json(&body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52501, msg = "GeTui auth error" }, "{}", _e);
                crate::Error::custom(52501, "GeTui auth request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52502, msg = "GeTui auth error" }, "{}", _e);
                crate::Error::custom(52502, "GeTui auth response parse failed")
            })?;

        if resp.code != 0 {
            #[cfg(feature = "log")]
            tracing::error!({ code = 42501, msg = "GeTui auth error" }, "code={}, msg={}", resp.code, resp.msg);
            return Err(crate::Error::custom(42501, &resp.msg));
        }

        let data = resp.data.ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 52502, msg = "GeTui auth error" }, "missing data");
            crate::Error::custom(52502, "GeTui auth response missing data")
        })?;

        // 使用返回的过期时间计算缓存（提前 1 小时刷新）
        let expire_ms: u64 = data.expire_time.parse().unwrap_or(86400000);
        let remaining_ms = expire_ms.saturating_sub(chrono::Utc::now().timestamp_millis() as u64);
        let cache_duration =
            std::time::Duration::from_millis(remaining_ms.saturating_sub(3600000).max(60000));
        let expires_at = std::time::Instant::now() + cache_duration;

        let mut cache = self.token_cache.lock().await;
        *cache = Some(CachedToken {
            token: data.token.clone(),
            expires_at,
        });

        Ok(data.token)
    }

    // ═══════════════════════════════════════════════════════════════
    //  toSingle 单推
    // ═══════════════════════════════════════════════════════════════

    /// 发送单条通知消息
    ///
    /// 自动选择接口：
    /// - `Cid` → `/push/single/cid`
    /// - `Alias` → `/push/single/alias`
    /// - `CidList` → `/push/single/batch/cid`（批量单推，≤200条）
    /// - `AliasList` → `/push/single/batch/alias`（批量单推，≤200条）
    /// - `Tag` → `/push/fast_custom_tag`
    /// - `All` → `/push/all`
    ///
    /// 自动处理 token 过期被动刷新（code=10001 时重试一次）。
    pub async fn send(
        &self,
        target: &PushTarget,
        notification: &Notification,
        settings: Option<&PushSettings>,
    ) -> crate::Result<PushResult> {
        let resp = self.do_send(target, notification, settings).await?;

        // 被动刷新：code=10001 表示 token 过期
        if resp.code == 10001 {
            #[cfg(feature = "log")]
            tracing::warn!("GeTui token expired, refreshing and retrying...");
            self.token_cache.lock().await.take();
            return self.do_send(target, notification, settings).await;
        }

        Ok(resp)
    }

    /// 发送透传消息
    ///
    /// 自动处理 token 过期重试。仅支持 `Cid`、`Alias`、`All` 目标。
    pub async fn send_transmission(
        &self,
        target: &PushTarget,
        content: &str,
        settings: Option<&PushSettings>,
    ) -> crate::Result<PushResult> {
        let resp = self.do_send_transmission(target, content, settings).await?;

        if resp.code == 10001 {
            #[cfg(feature = "log")]
            tracing::warn!("GeTui token expired, refreshing and retrying...");
            self.token_cache.lock().await.take();
            return self.do_send_transmission(target, content, settings).await;
        }

        Ok(resp)
    }

    // ═══════════════════════════════════════════════════════════════
    //  toList 批量推（两步）
    // ═══════════════════════════════════════════════════════════════

    /// 创建批量推消息体（toList 第一步）
    ///
    /// 返回 `taskid`，可用于多次调用 `send_list_cid` 或 `send_list_alias`。
    pub async fn create_list_message(
        &self,
        notification: &Notification,
        settings: Option<&PushSettings>,
        group_name: Option<&str>,
    ) -> crate::Result<String> {
        let token = self.get_token().await?;
        let url = format!("{}/push/list/message", self.base_url());

        let body = CreateMessageBody {
            group_name: group_name.map(|s| s.to_string()),
            settings: settings.cloned(),
            push_message: PushMessageBody {
                notification: Some(notification.clone()),
                transmission: None,
            },
        };

        let resp: CreateMessageResponse = self
            .client
            .post(&url)
            .header("Content-Type", "application/json;charset=utf-8")
            .header("token", &token)
            .json(&body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52503, msg = "GeTui create list message error" }, "{}", _e);
                crate::Error::custom(52503, "GeTui create list message request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52504, msg = "GeTui create list message error" }, "{}", _e);
                crate::Error::custom(52504, "GeTui create list message response parse failed")
            })?;

        if resp.code != 0 {
            #[cfg(feature = "log")]
            tracing::error!({ code = 42502, msg = "GeTui create list message error" }, "code={}, msg={}", resp.code, resp.msg);
            return Err(crate::Error::custom(42502, &resp.msg));
        }

        resp.data
            .ok_or_else(|| crate::Error::custom(52504, "missing taskid in response"))
            .map(|d| d.taskid)
    }

    /// 批量推 - CID 列表（toList 第二步）
    ///
    /// 先调用 `create_list_message` 获取 `taskid`。
    pub async fn send_list_cid(
        &self,
        taskid: &str,
        cids: &[&str],
        is_async: bool,
    ) -> crate::Result<PushResult> {
        let token = self.get_token().await?;
        let url = format!("{}/push/list/cid", self.base_url());

        let body = ListPushBody {
            audience: serde_json::json!({ "cid": cids }),
            taskid: taskid.to_string(),
            is_async: Some(is_async),
        };

        let resp: PushResult = self
            .client
            .post(&url)
            .header("Content-Type", "application/json;charset=utf-8")
            .header("token", &token)
            .json(&body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52503, msg = "GeTui list push error" }, "{}", _e);
                crate::Error::custom(52503, "GeTui list push request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52504, msg = "GeTui list push error" }, "{}", _e);
                crate::Error::custom(52504, "GeTui list push response parse failed")
            })?;

        Ok(resp)
    }

    /// 批量推 - 别名列表（toList 第二步）
    ///
    /// 先调用 `create_list_message` 获取 `taskid`。
    pub async fn send_list_alias(
        &self,
        taskid: &str,
        aliases: &[&str],
        is_async: bool,
    ) -> crate::Result<PushResult> {
        let token = self.get_token().await?;
        let url = format!("{}/push/list/alias", self.base_url());

        let body = ListPushBody {
            audience: serde_json::json!({ "alias": aliases }),
            taskid: taskid.to_string(),
            is_async: Some(is_async),
        };

        let resp: PushResult = self
            .client
            .post(&url)
            .header("Content-Type", "application/json;charset=utf-8")
            .header("token", &token)
            .json(&body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52503, msg = "GeTui list push error" }, "{}", _e);
                crate::Error::custom(52503, "GeTui list push request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52504, msg = "GeTui list push error" }, "{}", _e);
                crate::Error::custom(52504, "GeTui list push response parse failed")
            })?;

        Ok(resp)
    }

    // ═══════════════════════════════════════════════════════════════
    //  内部方法
    // ═══════════════════════════════════════════════════════════════

    /// 执行单推/群推（通知消息）
    async fn do_send(
        &self,
        target: &PushTarget,
        notification: &Notification,
        settings: Option<&PushSettings>,
    ) -> crate::Result<PushResult> {
        let token = self.get_token().await?;

        match target {
            // 单推：每个 cid/alias 独立请求
            PushTarget::Cid(cid) => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!({ "cid": [cid] }),
                    push_message: PushMessageBody {
                        notification: Some(notification.clone()),
                        transmission: None,
                    },
                };
                self.do_push(
                    &format!("{}/push/single/cid", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            PushTarget::Alias(alias) => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!({ "alias": [alias] }),
                    push_message: PushMessageBody {
                        notification: Some(notification.clone()),
                        transmission: None,
                    },
                };
                self.do_push(
                    &format!("{}/push/single/alias", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            // 批量单推：同一消息发给多个 cid/alias，≤200条
            PushTarget::CidList(cids) => {
                let msg_list: Vec<PushRequestBody> = cids
                    .iter()
                    .map(|cid| PushRequestBody {
                        request_id: uuid::new(),
                        group_name: None,
                        settings: settings.cloned(),
                        audience: serde_json::json!({ "cid": [cid] }),
                        push_message: PushMessageBody {
                            notification: Some(notification.clone()),
                            transmission: None,
                        },
                    })
                    .collect();
                let body = BatchPushRequestBody {
                    is_async: Some(false),
                    msg_list,
                };
                self.do_batch_push(
                    &format!("{}/push/single/batch/cid", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            PushTarget::AliasList(aliases) => {
                let msg_list: Vec<PushRequestBody> = aliases
                    .iter()
                    .map(|alias| PushRequestBody {
                        request_id: uuid::new(),
                        group_name: None,
                        settings: settings.cloned(),
                        audience: serde_json::json!({ "alias": [alias] }),
                        push_message: PushMessageBody {
                            notification: Some(notification.clone()),
                            transmission: None,
                        },
                    })
                    .collect();
                let body = BatchPushRequestBody {
                    is_async: Some(false),
                    msg_list,
                };
                self.do_batch_push(
                    &format!("{}/push/single/batch/alias", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            // 标签快速推送
            PushTarget::Tag(tag) => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!({ "fast_custom_tag": tag }),
                    push_message: PushMessageBody {
                        notification: Some(notification.clone()),
                        transmission: None,
                    },
                };
                self.do_push(
                    &format!("{}/push/fast_custom_tag", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            // 全量推送
            PushTarget::All => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!("all"),
                    push_message: PushMessageBody {
                        notification: Some(notification.clone()),
                        transmission: None,
                    },
                };
                self.do_push(&format!("{}/push/all", self.base_url()), &body, &token)
                    .await
            }
        }
    }

    /// 执行透传消息推送
    async fn do_send_transmission(
        &self,
        target: &PushTarget,
        content: &str,
        settings: Option<&PushSettings>,
    ) -> crate::Result<PushResult> {
        let token = self.get_token().await?;

        match target {
            PushTarget::Cid(cid) => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!({ "cid": [cid] }),
                    push_message: PushMessageBody {
                        notification: None,
                        transmission: Some(content.to_string()),
                    },
                };
                self.do_push(
                    &format!("{}/push/single/cid", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            PushTarget::Alias(alias) => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!({ "alias": [alias] }),
                    push_message: PushMessageBody {
                        notification: None,
                        transmission: Some(content.to_string()),
                    },
                };
                self.do_push(
                    &format!("{}/push/single/alias", self.base_url()),
                    &body,
                    &token,
                )
                .await
            }
            PushTarget::All => {
                let body = PushRequestBody {
                    request_id: uuid::new(),
                    group_name: None,
                    settings: settings.cloned(),
                    audience: serde_json::json!("all"),
                    push_message: PushMessageBody {
                        notification: None,
                        transmission: Some(content.to_string()),
                    },
                };
                self.do_push(&format!("{}/push/all", self.base_url()), &body, &token)
                    .await
            }
            _ => Err(crate::Error::custom(
                42502,
                "透传消息仅支持 Cid、Alias、All 目标",
            )),
        }
    }

    /// 执行单推请求
    async fn do_push(
        &self,
        url: &str,
        body: &PushRequestBody,
        token: &str,
    ) -> crate::Result<PushResult> {
        let resp: PushResult = self
            .client
            .post(url)
            .header("Content-Type", "application/json;charset=utf-8")
            .header("token", token)
            .json(body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52503, msg = "GeTui push error" }, "{}", _e);
                crate::Error::custom(52503, "GeTui push request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52504, msg = "GeTui push error" }, "{}", _e);
                crate::Error::custom(52504, "GeTui push response parse failed")
            })?;

        if resp.code != 0 && resp.code != 10001 {
            #[cfg(feature = "log")]
            tracing::warn!({ code = 42502, msg = "GeTui push error" }, "code={}, msg={}", resp.code, resp.msg);
        }

        Ok(resp)
    }

    /// 执行批量单推请求
    async fn do_batch_push(
        &self,
        url: &str,
        body: &BatchPushRequestBody,
        token: &str,
    ) -> crate::Result<PushResult> {
        let resp: PushResult = self
            .client
            .post(url)
            .header("Content-Type", "application/json;charset=utf-8")
            .header("token", token)
            .json(body)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52503, msg = "GeTui batch push error" }, "{}", _e);
                crate::Error::custom(52503, "GeTui batch push request failed")
            })?
            .json()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 52504, msg = "GeTui batch push error" }, "{}", _e);
                crate::Error::custom(52504, "GeTui batch push response parse failed")
            })?;

        if resp.code != 0 && resp.code != 10001 {
            #[cfg(feature = "log")]
            tracing::warn!({ code = 42502, msg = "GeTui batch push error" }, "code={}, msg={}", resp.code, resp.msg);
        }

        Ok(resp)
    }
}

// ═══════════════════════════════════════════════════════════════
//  UUID 生成（简易版）
// ═══════════════════════════════════════════════════════════════

mod uuid {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// 生成 32 位唯一 ID（时间戳 + 计数器）
    pub fn new() -> String {
        let ts = chrono::Utc::now().timestamp_millis();
        let cnt = COUNTER.fetch_add(1, Ordering::Relaxed);
        format!("{:013}{:019}", ts, cnt)
    }
}
