use serde::Deserialize;

use super::WxWebLoginResult;
use super::err;
use crate::AppState;
use crate::state::callbacks::AsyncCallback;

// ═══════════════════════════════════════════════════════════════
//  回调类型
// ═══════════════════════════════════════════════════════════════

/// 微信网页登录回调函数类型
///
/// 接收 `AppState`、`WxWebLoginResult` 和 OAuth2 `state` 参数，
/// 返回 `WxWebLoginResult`。
pub type WxWebLoginCallbackFn =
    AsyncCallback<(AppState, WxWebLoginResult, Option<String>), crate::Result<WxWebLoginResult>>;

// ═══════════════════════════════════════════════════════════════
//  请求类型
// ═══════════════════════════════════════════════════════════════

/// 微信网页登录回调查询参数
#[derive(Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("微信网页登录回调查询参数")]
pub struct WxWebCallbackQuery {
    /// 授权码
    pub code: Option<String>,
    /// CSRF 防护状态码
    pub state: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  Handler
// ═══════════════════════════════════════════════════════════════

/// 微信网页扫码登录回调处理器
///
/// 自动完成以下流程：
/// 1. 从查询参数中提取 `code` 和 `state`
/// 2. 调用微信 API 通过 code 换取 access_token
/// 3. 获取用户信息
/// 4. 调用用户注册的回调函数处理业务逻辑
/// 5. 以 JSON 格式返回 `WxWebLoginResult`
///
/// 通过 `#[afast::get]` 自动注册为 ordinary-http GET 端点
#[afast::get(desc("微信网页扫码登录回调"))]
async fn callback(
    afast::State(state): afast::State<AppState>,
    afast::Query(query): afast::Query<WxWebCallbackQuery>,
) -> afast::Result<afast::Json<WxWebLoginResult>> {
    let code = query.code.ok_or_else(|| {
        #[cfg(feature = "log")]
        tracing::warn!(
            { code = 42102, msg = "WeChat web login callback error: Missing code parameter" },
            "missing code parameter"
        );
        err::web_no_code()
    })?;

    let oauth_state = query.state;

    // 完整登录流程：换取 access_token + 获取用户信息
    let result = state.wxlogin.web_login_full(&code).await?;

    let cb = state.wxlogin.web_login_callback.clone().ok_or_else(|| {
        #[cfg(feature = "log")]
        tracing::error!(
            { code = 52111, msg = "WeChat web login callback error: Callback not registered" },
            "WeChat web login callback not configured"
        );
        err::web_no_callback()
    })?;

    let user_result = cb((state.clone(), result, oauth_state)).await?;
    Ok(afast::Json(user_result))
}
