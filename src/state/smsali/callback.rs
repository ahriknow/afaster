use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::state::callbacks::AsyncCallback;

// ═══════════════════════════════════════════════════════════════
//  回调类型
// ═══════════════════════════════════════════════════════════════

/// 阿里云短信回执回调函数类型
///
/// 接收 `(AppState, Vec<AliSmsReport>)`，
/// 返回 `AliSmsCallbackResult`。
pub type AliSmsReportCallbackFn =
    AsyncCallback<(AppState, Vec<AliSmsReport>), crate::Result<AliSmsCallbackResult>>;

// ═══════════════════════════════════════════════════════════════
//  请求/响应类型
// ═══════════════════════════════════════════════════════════════

/// 阿里云短信回执状态报告
///
/// 参考: https://help.aliyun.com/zh/sms/developer-reference/receipts/
#[derive(Debug, Clone, Deserialize, afast::AFastDeserialize, afast::Tag)]
#[serde(rename_all = "camelCase")]
#[tag("阿里云短信回执状态报告")]
pub struct AliSmsReport {
    /// 发送回执 ID（与 SendSms 返回的 BizId 对应）
    pub biz_id: String,
    /// 手机号
    pub phone_number: String,
    /// 发送时间
    pub send_time: String,
    /// 运营商回执时间
    pub report_time: String,
    /// 是否成功送达
    pub success: bool,
    /// 运营商错误码（成功时为空字符串）
    #[serde(default)]
    pub err_code: String,
    /// 运营商错误描述
    #[serde(default)]
    pub err_msg: String,
    /// 短信模板 Code
    #[serde(default)]
    pub template_code: String,
    /// 短信签名
    #[serde(default)]
    pub sign_name: String,
    /// 上行短信扩展码（SmsUp 类型时存在）
    #[serde(default)]
    pub dest_code: String,
    /// 上行短信内容（SmsUp 类型时存在）
    #[serde(default)]
    pub content: String,
}

/// 阿里云短信回调响应
#[derive(Debug, Clone, Serialize, afast::Tag)]
#[tag("阿里云短信回调响应")]
pub struct AliSmsCallbackResult {
    pub code: i32,
    pub msg: String,
}

impl AliSmsCallbackResult {
    pub fn ok() -> Self {
        Self {
            code: 0,
            msg: "ok".to_string(),
        }
    }
}

/// 回执报告批量包装（用于 Body 反序列化）
///
/// 阿里云推送的 body 为 JSON 数组，`Body<T>` 要求 `T: Structure`，
/// 而 `Vec<T>` 未实现该 trait，故使用 newtype 包装并手动实现。
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct AliSmsReportBatch {
    pub reports: Vec<AliSmsReport>,
}

impl afast::Structure for AliSmsReportBatch {
    fn structure() -> &'static afast::handler::TagMeta {
        use afast::handler::{TagKind, TagMeta};
        static META: TagMeta = TagMeta {
            name: "AliSmsReportBatch",
            desc: "阿里云短信回执报告列表",
            kind: TagKind::Struct(&[]),
        };
        &META
    }
}

// ═══════════════════════════════════════════════════════════════
//  Handler
// ═══════════════════════════════════════════════════════════════

/// 阿里云短信回执回调处理器
///
/// 自动完成以下流程：
/// 1. 从 POST Body 解析回执报告列表
/// 2. 调用用户注册的回调函数处理业务逻辑
/// 3. 返回 `{"code": 0, "msg": "ok"}` 确认接收
///
/// 通过 `#[afast::post]` 自动注册为 ordinary-http POST 端点
///
/// 阿里云回执推送支持两种类型：
/// - SmsReport：下行状态报告（每条短信是否送达）
/// - SmsUp：上行短信（用户回复内容）
///
/// 两种类型的 body 结构相同，通过 `dest_code` 字段区分。
#[afast::post(desc("阿里云短信回执回调"))]
async fn callback(
    afast::State(state): afast::State<AppState>,
    afast::Body(batch): afast::Body<AliSmsReportBatch>,
) -> afast::Result<afast::Json<AliSmsCallbackResult>> {
    let reports = batch.reports;
    let Some(cb) = state.sms_ali.sms_report_callback.clone() else {
        // 未注册回调 → 静默丢弃，返回 OK 避免云平台重试
        #[cfg(feature = "log")]
        tracing::debug!(
            count = reports.len(),
            "Alibaba SMS report received but no callback registered, discarding"
        );
        return Ok(afast::Json(AliSmsCallbackResult::ok()));
    };

    let result = cb((state.clone(), reports)).await?;
    Ok(afast::Json(result))
}
