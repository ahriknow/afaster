#![allow(dead_code)]

#[cfg(feature = "push-getui")]
pub mod getui;
#[cfg(feature = "push-getui")]
use getui::GeTui;

#[cfg(feature = "push-jpush")]
pub mod jpush;
#[cfg(feature = "push-jpush")]
use jpush::JPush;

#[cfg(feature = "push-xiaomi")]
pub mod xiaomi;
#[cfg(feature = "push-xiaomi")]
use xiaomi::XiaomiPush;

use serde::Deserialize;

// ═══════════════════════════════════════════════════════════════
//  推送公共类型
// ═══════════════════════════════════════════════════════════════

/// 推送平台
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// 全平台
    All,
    /// iOS
    Ios,
    /// Android
    Android,
    /// HarmonyOS
    Harmony,
}

/// 推送目标
#[derive(Debug, Clone)]
pub enum PushTarget {
    /// 单个 CID
    Cid(String),
    /// 多个 CID
    CidList(Vec<String>),
    /// 单个别名
    Alias(String),
    /// 多个别名
    AliasList(Vec<String>),
    /// 标签快速推送
    Tag(String),
    /// 全量推送
    All,
}

/// 通知消息
#[derive(Debug, Clone, serde::Serialize)]
pub struct Notification {
    /// 通知标题（≤50字）
    pub title: String,
    /// 通知内容（≤256字）
    pub body: String,
    /// 点击后续动作
    #[serde(skip_serializing_if = "Option::is_none")]
    pub click_type: Option<String>,
    /// 网页地址（click_type=url 时必填）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// 长文本（≤512字，与 big_image 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub big_text: Option<String>,
    /// 大图 URL（≤1024字，与 big_text 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub big_image: Option<String>,
    /// 通知图标 URL（≤256字）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_url: Option<String>,
}

impl Notification {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            click_type: None,
            url: None,
            big_text: None,
            big_image: None,
            logo_url: None,
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.click_type = Some("url".to_string());
        self.url = Some(url.into());
        self
    }

    pub fn start_app(mut self) -> Self {
        self.click_type = Some("startapp".to_string());
        self
    }

    pub fn none(mut self) -> Self {
        self.click_type = Some("none".to_string());
        self
    }
}

/// 推送设置
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PushSettings {
    /// 消息离线时间（毫秒），默认 2 小时，-1 不设离线
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    /// 定速推送（条/秒），0 不限速
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<i32>,
    /// 定时推送（毫秒时间戳，7天内）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule_time: Option<i64>,
}

/// 推送结果
#[derive(Debug, Clone, serde::Deserialize)]
pub struct PushResult {
    /// 错误码，0 表示成功
    pub code: i32,
    /// 错误信息
    pub msg: String,
    /// 返回数据
    #[cfg(feature = "push-getui")]
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// ═══════════════════════════════════════════════════════════════
//  推送管理器
// ═══════════════════════════════════════════════════════════════

/// 推送管理器
///
/// 根据启用的 feature 自动初始化对应推送通道。
///
/// ```toml
/// # Cargo.toml
/// [dependencies]
/// afaster = { features = ["push-getui"] }
///
/// # config.toml
/// [push.getui]
/// app_id = ""
/// app_key = ""
/// master_secret = ""
/// ```
///
/// ```rust,no_run
/// // 自动可用
/// let result = state.push.send_getui(&target, &notification).await?;
/// ```
#[derive(Clone, Deserialize)]
pub struct PushManager {
    #[cfg(feature = "push-getui")]
    pub getui: GeTui,
    #[cfg(feature = "push-jpush")]
    pub jpush: JPush,
    #[cfg(feature = "push-xiaomi")]
    pub xiaomi: XiaomiPush,
}

impl PushManager {
    /// 初始化（内部调用）
    pub fn init(&mut self) {
        #[cfg(feature = "push-getui")]
        {
            self.getui.client = super::default_http_client();
        }
        #[cfg(feature = "push-jpush")]
        {
            self.jpush.client = super::default_http_client();
        }
        #[cfg(feature = "push-xiaomi")]
        {
            self.xiaomi.client = super::default_http_client();
        }
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "push")?;
        instance.init();
        Ok(instance)
    }
}
