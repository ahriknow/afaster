use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::state::callbacks::AsyncCallback;

// ═══════════════════════════════════════════════════════════════
//  回调类型
// ═══════════════════════════════════════════════════════════════

/// 腾讯云短信回执回调函数类型
///
/// 接收 `(AppState, Vec<TencentSmsReport>)`，
/// 返回 `TencentSmsCallbackResult`。
pub type TencentSmsReportCallbackFn =
    AsyncCallback<(AppState, Vec<TencentSmsReport>), crate::Result<TencentSmsCallbackResult>>;

// ═══════════════════════════════════════════════════════════════
//  请求/响应类型
// ═══════════════════════════════════════════════════════════════

/// 腾讯云短信回执状态报告
///
/// 参考: https://cloud.tencent.com/document/product/382/59178
///
/// 回调 body 为 JSON 数组，一次回调可能包含多条报告。
#[derive(Debug, Clone, Deserialize, afast::AFastDeserialize, afast::Tag)]
#[serde(rename_all = "snake_case")]
#[tag("腾讯云短信回执状态报告")]
pub struct TencentSmsReport {
    /// 用户实际接收到短信的时间
    #[serde(default)]
    pub user_receive_time: String,
    /// 国家（或地区）码
    #[serde(default)]
    pub nationcode: String,
    /// 手机号码
    pub mobile: String,
    /// 实际送达状态：SUCCESS / FAIL
    pub report_status: String,
    /// 用户接收短信状态码错误信息
    #[serde(default)]
    pub errmsg: String,
    /// 用户接收短信状态描述
    #[serde(default)]
    pub description: String,
    /// 本次发送标识 ID（与发送接口返回的 SerialNo 对应）
    #[serde(default)]
    pub sid: String,
    /// 用户的 session 内容（与发送接口的 SessionContext 一致）
    #[serde(default)]
    pub ext: String,
}

/// 腾讯云短信回调响应
///
/// 参考: https://cloud.tencent.com/document/product/382/59178
#[derive(Debug, Clone, Serialize, afast::Tag)]
#[tag("腾讯云短信回调响应")]
pub struct TencentSmsCallbackResult {
    pub result: i32,
    pub errmsg: String,
}

impl TencentSmsCallbackResult {
    pub fn ok() -> Self {
        Self {
            result: 0,
            errmsg: "OK".to_string(),
        }
    }
}

/// 回执报告批量包装（用于 Body 反序列化）
///
/// 腾讯云推送的 body 为 JSON 数组，`Body<T>` 要求 `T: Structure`，
/// 而 `Vec<T>` 未实现该 trait，故使用 newtype 包装并手动实现。
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct TencentSmsReportBatch {
    pub reports: Vec<TencentSmsReport>,
}

impl afast::Structure for TencentSmsReportBatch {
    fn structure() -> &'static afast::handler::TagMeta {
        use afast::handler::{TagKind, TagMeta};
        static META: TagMeta = TagMeta {
            name: "TencentSmsReportBatch",
            desc: "腾讯云短信回执报告列表",
            kind: TagKind::Struct(&[]),
        };
        &META
    }
}

// ═══════════════════════════════════════════════════════════════
//  Handler
// ═══════════════════════════════════════════════════════════════

/// 腾讯云短信回执回调处理器
///
/// 自动完成以下流程：
/// 1. 从 POST Body 解析回执报告列表（JSON 数组）
/// 2. 调用用户注册的回调函数处理业务逻辑
/// 3. 返回 `{"result": 0, "errmsg": "OK"}` 确认接收
///
/// 通过 `#[afast::post]` 自动注册为 ordinary-http POST 端点
#[afast::post(desc("腾讯云短信回执回调"))]
async fn callback(
    afast::State(state): afast::State<AppState>,
    afast::Body(batch): afast::Body<TencentSmsReportBatch>,
) -> afast::Result<afast::Json<TencentSmsCallbackResult>> {
    let reports = batch.reports;
    let Some(cb) = state.sms_tencent.sms_report_callback.clone() else {
        // 未注册回调 → 静默丢弃，返回 OK 避免云平台重试
        #[cfg(feature = "log")]
        tracing::debug!(
            count = reports.len(),
            "Tencent SMS report received but no callback registered, discarding"
        );
        return Ok(afast::Json(TencentSmsCallbackResult::ok()));
    };

    let result = cb((state.clone(), reports)).await?;
    Ok(afast::Json(result))
}
