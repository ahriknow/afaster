use serde::Deserialize;

use super::WxMpLoginResult;
use super::err;
use crate::AppState;
use crate::state::callbacks::AsyncCallback;

// ═══════════════════════════════════════════════════════════════
//  回调类型
// ═══════════════════════════════════════════════════════════════

/// 微信公众号网页授权回调函数类型
pub type WxMpLoginCallbackFn =
    AsyncCallback<(AppState, WxMpLoginResult, Option<String>), crate::Result<WxMpLoginResult>>;

// ═══════════════════════════════════════════════════════════════
//  请求类型
// ═══════════════════════════════════════════════════════════════

/// 微信公众号网页授权回调查询参数
#[derive(Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("微信公众号网页授权回调查询参数")]
pub struct WxMpCallbackQuery {
    /// 授权码
    pub code: Option<String>,
    /// CSRF 防护状态码
    pub state: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  Handler
// ═══════════════════════════════════════════════════════════════

/// 微信公众号网页授权回调处理器
#[afast::get(desc("微信公众号网页授权回调"))]
async fn callback(
    afast::State(state): afast::State<AppState>,
    afast::Query(query): afast::Query<WxMpCallbackQuery>,
) -> afast::Result<afast::Json<WxMpLoginResult>> {
    let code = query.code.ok_or_else(|| {
        #[cfg(feature = "log")]
        tracing::warn!(
            { code = 42202, msg = "WeChat mp login callback error: Missing code parameter" },
            "missing code parameter"
        );
        err::mp_no_code()
    })?;

    let oauth_state = query.state;

    // 完整登录流程：换取 access_token + 获取用户信息
    let result = state.wx_official.mp_login_full(&code).await?;

    let cb = state.wx_official.mp_login_callback.clone().ok_or_else(|| {
        #[cfg(feature = "log")]
        tracing::error!(
            { code = 52211, msg = "WeChat mp login callback error: Callback not registered" },
            "WeChat mp login callback not configured"
        );
        err::mp_no_callback()
    })?;

    let user_result = cb((state.clone(), result, oauth_state)).await?;
    Ok(afast::Json(user_result))
}
