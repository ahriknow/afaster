use serde::Serialize;

// ═══════════════════════════════════════════════════════════════
//  响应类型
// ═══════════════════════════════════════════════════════════════

/// 推送响应
///
/// 微信要求推送响应格式为：
/// - JSON: `{"ErrCode":0,"ErrMsg":"success"}`
#[derive(Debug, Serialize, afast::Tag)]
pub struct WxSecCheckNotifyResult {
    #[serde(rename = "ErrCode")]
    pub err_code: i32,
    #[serde(rename = "ErrMsg")]
    pub err_msg: String,
}

impl WxSecCheckNotifyResult {
    /// 成功响应
    pub fn success() -> Self {
        Self {
            err_code: 0,
            err_msg: "success".to_string(),
        }
    }

    /// 失败响应
    pub fn error(err_code: i32, err_msg: impl Into<String>) -> Self {
        Self {
            err_code,
            err_msg: err_msg.into(),
        }
    }
}

use crate::state::callbacks::AsyncCallback;

/// 媒体检测异步推送回调函数类型
pub type MediaCheckCallbackFn =
    AsyncCallback<(crate::AppState, MediaCheckNotify), crate::Result<WxSecCheckNotifyResult>>;

/// 媒体检测通知
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaCheckNotify {
    pub to_user_name: Option<String>,
    pub from_user_name: Option<String>,
    pub create_time: Option<i64>,
    pub msg_type: Option<String>,
    pub event: Option<String>,
    pub appid: Option<String>,
    pub trace_id: Option<String>,
    pub version: Option<String>,
    pub detail: Option<Vec<MediaCheckDetail>>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub retry_times: Option<i32>,
}

/// 媒体检测详情
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MediaCheckDetail {
    pub strategy: Option<String>,
    pub errcode: Option<i32>,
    pub suggest: Option<String>,
    pub label: Option<i32>,
    pub prob: Option<f64>,
    pub sub: Option<Vec<MediaCheckSubDetail>>,
}

/// 媒体检测子详情
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MediaCheckSubDetail {
    pub strategy: Option<String>,
    pub errcode: Option<i32>,
    pub suggest: Option<String>,
    pub label: Option<i32>,
    pub prob: Option<f64>,
}

// ═══════════════════════════════════════════════════════════════
//  自动注册的回调 Handler
// ═══════════════════════════════════════════════════════════════

/// 微信内容安全 GET 验签处理器
#[afast::get(desc("微信内容安全 - 服务器验签"))]
pub async fn verify(
    afast::State(state): afast::State<crate::AppState>,
    afast::Query(query): afast::Query<super::notify::WxVerifyQuery>,
) -> afast::Result<afast::Text> {
    let token = &state.wx_sec_check.verify_token;
    if token.is_empty() {
        return Err(afast::Error::custom(40801, "missing verify param"));
    }
    super::notify::verify_signature(token, &query.timestamp, &query.nonce, &query.signature)?;
    Ok(afast::Text(query.echostr))
}

/// 微信内容安全 POST 通知分发处理器
#[afast::post(desc("微信内容安全 - 通知回调"))]
pub async fn dispatch(
    afast::State(state): afast::State<crate::AppState>,
    afast::Body(body): afast::Body<super::notify::WxNotifyBody>,
) -> afast::Result<afast::Text> {
    Ok(state
        .wx_sec_check
        .handle_notify(state.clone(), body)
        .await?)
}
