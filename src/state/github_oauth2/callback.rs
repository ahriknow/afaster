use serde::Deserialize;

use super::GitHubOAuth2Result;
use super::err;
use crate::AppState;
use crate::state::callbacks::AsyncCallback;

// ═══════════════════════════════════════════════════════════════
//  回调类型
// ═══════════════════════════════════════════════════════════════

/// GitHub OAuth2 回调函数类型
///
/// 接收 `AppState`、`GitHubOAuth2Result` 和 OAuth2 `state` 参数，
/// 返回 `GitHubOAuth2Result`。
/// `state` 始终传递，无论是否启用内置验证。
pub type GitHubOAuth2CallbackFn = AsyncCallback<
    (AppState, GitHubOAuth2Result, Option<String>),
    crate::Result<GitHubOAuth2Result>,
>;

// ═══════════════════════════════════════════════════════════════
//  请求类型
// ═══════════════════════════════════════════════════════════════

/// GitHub OAuth2 回调查询参数
#[derive(Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("GitHub OAuth2 回调查询参数")]
pub struct GitHubCallbackQuery {
    /// 授权码
    pub code: Option<String>,
    /// CSRF 防护状态码
    pub state: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  Handler
// ═══════════════════════════════════════════════════════════════

/// GitHub OAuth2 回调处理器
///
/// 自动完成以下流程：
/// 1. 从查询参数中提取 `code`
/// 2. 调用 GitHub API 交换 Token 并获取用户信息
/// 3. 调用用户注册的回调函数处理业务逻辑
/// 4. 以 JSON 格式返回 `GitHubOAuth2Result`
///
/// 通过 `#[afast::get]` 自动注册为 ordinary-http GET 端点
#[afast::get(desc("GitHub OAuth2 回调"))]
async fn callback(
    afast::State(state): afast::State<AppState>,
    afast::Query(query): afast::Query<GitHubCallbackQuery>,
) -> afast::Result<afast::Json<GitHubOAuth2Result>> {
    let code = query.code.ok_or_else(|| {
        #[cfg(feature = "log")]
        tracing::warn!(
            { code = 40503, msg = "GitHub OAuth2 callback error: Missing code parameter" },
            "missing code parameter"
        );
        err::no_code()
    })?;

    let oauth_state = query.state;

    // 内置 state 验证
    if state.github_oauth2.verify_state {
        match oauth_state.as_deref() {
            None => {
                #[cfg(feature = "log")]
                tracing::warn!(
                    { code = 40504, msg = "GitHub OAuth2 callback error: Missing state parameter" },
                    "missing state parameter"
                );
                return Err(err::no_state().into());
            }
            Some(s) if !state.github_oauth2.validate_state(s) => {
                #[cfg(feature = "log")]
                tracing::warn!(
                    { code = 40505, msg = "GitHub OAuth2 callback error: Invalid state" },
                    "invalid or expired state"
                );
                return Err(err::invalid_state().into());
            }
            _ => {} // 验证通过
        }
    }

    let result = state.github_oauth2.login(&code).await?;

    let cb = state
        .github_oauth2
        .github_oauth2_callback
        .clone()
        .ok_or_else(|| {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 50507, msg = "GitHub OAuth2 callback error: Callback not registered" },
                "GitHub callback not configured"
            );
            err::no_callback()
        })?;

    let user_result = cb((state.clone(), result, oauth_state)).await?;
    Ok(afast::Json(user_result))
}
