# GitHub OAuth2

Feature: `github-oauth2`（自动启用 `nonce`）

> 📖 官方文档：<https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps>

## 配置

```toml
[github_oauth2]
client_id = ""                                                    # 必填
client_secret = ""                                                # 必填
redirect_base = "http://localhost:5000"                           # 必填, 服务基础地址
callback_path = "auth/github/callback"                            # 可选, 默认 "auth/github/callback"
scope = "read:user user:email"                                    # 可选
authorize_url = "https://github.com/login/oauth/authorize"        # 可选
token_url = "https://github.com/login/oauth/access_token"         # 可选
user_url = "https://api.github.com/user"                          # 可选
verify_state = false                                              # 可选, 启用框架内置 state 验证 (需 nonce feature)
state_expire = 300                                                # 可选, state 有效期秒数, 默认 300
```

## API

### GitHubOAuth2

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_authorize_url` | `state: Option<&str>` | `String` | 生成授权 URL（手动传 state） |
| `generate_authorize_url` | `nonce: &Nonce` | `String` | 生成带签名 state 的授权 URL（`verify_state=true` 时推荐） |
| `generate_state` | `nonce: &Nonce` | `String` | 生成签名 state 值 |
| `validate_state` | `state: &str` | `bool` | 验证 state 签名和有效期 |
| `is_verify_state` | — | `bool` | 是否启用了内置验证 |
| `exchange_code` | `code: &str` | `Result<GitHubTokenResponse>` | 授权码换 Token |
| `get_user` | `access_token: &str` | `Result<GitHubUser>` | 获取用户信息 |
| `get_user_emails` | `access_token: &str` | `Result<Vec<GitHubEmail>>` | 获取用户邮箱列表 |
| `login` | `code: &str` | `Result<GitHubOAuth2Result>` | 完整登录流程 |


### GitHubOAuth2Result

```rust
pub struct GitHubOAuth2Result {
    pub github_id: i64,
    pub username: String,
    pub display_name: Option<String>,    // read:user
    pub email: Option<String>,           // user:email
    pub avatar_url: Option<String>,      // read:user
    pub profile_url: Option<String>,     // read:user
    pub bio: Option<String>,             // read:user
    pub company: Option<String>,         // read:user
    pub blog: Option<String>,            // read:user
    pub location: Option<String>,        // read:user
    pub twitter_username: Option<String>, // read:user
}
```

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 40501 | GitHub authentication failed | GitHub 认证失败 |
| 40502 | No access_token obtained | 未获取到 access_token |
| 40503 | Missing authorization code | 缺少授权码 |
| 40504 | Missing state parameter | 缺少 state 参数 |
| 40505 | Invalid or expired state | state 验证失败 |
| 50501 | Token exchange request failed | Token 请求发送失败 |
| 50502 | Token exchange response parse failed | Token 响应解析失败 |
| 50503 | User info request failed | 获取用户信息请求失败 |
| 50504 | User info response parse failed | 用户信息解析失败 |
| 50505 | User email request failed | 获取用户邮箱请求失败 |
| 50506 | User email response parse failed | 用户邮箱解析失败 |
| 50507 | GitHub callback not registered | 未注册回调函数 |

## OAuth2 流程

```
1. 前端重定向 → generate_authorize_url(&nonce)  // 自动带签名 state
2. 用户授权 → GitHub 回调 redirect_base/callback_path?code=xxx&state=yyy
3. 框架自动验证 state（verify_state=true 时）
4. 框架调用 login(&code) 交换 Token + 获取用户信息
5. 调用用户注册的回调，传入 (state, result, oauth_state)
6. 返回用户处理后的 result
```

## 回调系统

开启 `github-oauth2` feature 后，回调路由自动注册为 GET 端点，无需手动注册 handler。
路由路径由配置中的 `callback_path` 决定，默认 `auth/github/callback`。

### 注册回调

回调接收三个参数：`AppState`、`GitHubOAuth2Result` 和 OAuth2 `state`（`Option<String>`）。

```rust
AFaster::new()
    .config("config.toml")
    .with_github_oauth2(|g| {
        g.with_github_callback(|(state, result, oauth_state)| {
            async move {
                // result: GitHubOAuth2Result（GitHub 用户信息）
                // oauth_state: Option<String>（OAuth2 state 参数，始终传递）
                println!("GitHub ID: {}", result.github_id);
                println!("Username: {}", result.username);
                if let Some(s) = oauth_state {
                    println!("State: {}", s);
                }
                // 处理业务逻辑，返回处理后的 result
                Ok(result)
            }
        })
    })
    .run()
    .await;
```

### State 验证

配置 `verify_state = true` 让框架自动验证 state：

```toml
[github_oauth2]
verify_state = true   # 启用内置 state 验证
state_expire = 300    # state 有效期 300 秒
```

框架使用 HMAC-SHA256 签名方案（无状态，无需存储）：
- `generate_authorize_url(&nonce)` 自动生成 `{random}.{timestamp}.{hmac_sig}` 格式的签名 state
- 回调时自动验证签名和有效期，不通过返回 40504/40505 错误
- state 始终传递给用户回调，无论是否启用内置验证

## 使用示例

```rust
// 1. 生成授权 URL（推荐，自动处理 state）
let url = state.github_oauth2.generate_authorize_url(&state.nonce);

// 2. 手动生成（自定义 state）
let url = state.github_oauth2.get_authorize_url(Some("my_state"));

// 3. 完整登录流程（handler 中自动完成 code 交换 + 获取用户信息 + 调用回调）
// 由 callback handler 自动处理，无需手动调用

// 4. 手动调用（不使用回调系统时）
let result = state.github_oauth2.login(&code).await?;
println!("GitHub ID: {}", result.github_id);
println!("Username: {}", result.username);
println!("Email: {:?}", result.email);
```
