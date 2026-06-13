# 微信登录

> Feature: `wx-login-mini` / `wx-login-app` / `wx-login-web`

统一的微信登录模块，支持小程序、APP、网页扫码三种登录方式，共享 `[wxlogin]` 配置段。

> 公众号网页授权登录已合并到 `wx-official` 模块，详见 [wx-official.md](wx-official.md)。

## 配置

```toml
[wxlogin]
# 小程序登录
mini_id = "wx1234567890"
mini_secret = ""

# APP 登录
app_id = "wx1234567890"
app_secret = ""

# 网页扫码登录
web_id = ""
web_secret = ""
redirect_base = "https://example.com"
# callback_path = "auth/wechat/callback"  # 默认值，框架自动注册回调路由
```

各登录方式只需配置对应的字段即可，未启用的 feature 对应的字段可省略。

---

## 小程序登录

Feature: `wx-login-mini`

> 📖 官方文档：<https://developers.weixin.qq.com/miniprogram/dev/api-backend/open-api/login/auth.code2Session.html>

小程序前端通过 `wx.login()` 获取 `code`，后端调用 `mini_login` 换取 `openid` 和 `session_key`。

### 使用示例

```rust
let result = state.wxlogin.mini_login(&code).await?;
let openid = result.openid;
let session_key = result.session_key;
```

### API

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `mini_login` | `code: &str` | `Result<MiniLoginResponse>` | 小程序登录 |

### MiniLoginResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `openid` | `String` | 用户唯一标识 |
| `session_key` | `String` | 会话密钥 |
| `unionid` | `Option<String>` | 统一标识（绑定了开放平台才有） |
| `errcode` | `Option<i32>` | 错误码 |
| `errmsg` | `Option<String>` | 错误信息 |

---

## APP 登录

Feature: `wx-login-app`

> 📖 官方文档：<https://developers.weixin.qq.com/doc/oplatform/Mobile_App/operation.html>

APP 端通过微信 SDK 获取 `code`，后端调用 `app_login` 换取 `access_token` 和 `openid`。

### 使用示例

```rust
let result = state.wxlogin.app_login(&code).await?;
let access_token = result.access_token;
let openid = result.openid;
```

### API

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `app_login` | `code: &str` | `Result<AppLoginResponse>` | APP 登录 |

### AppLoginResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `access_token` | `String` | 接口调用凭证 |
| `expires_in` | `i32` | 有效期（秒） |
| `refresh_token` | `String` | 刷新凭证 |
| `openid` | `String` | 用户唯一标识 |
| `scope` | `String` | 授权作用域 |
| `unionid` | `Option<String>` | 统一标识 |
| `errcode` | `Option<i32>` | 错误码 |
| `errmsg` | `Option<String>` | 错误信息 |

---

## 网页扫码登录

Feature: `wx-login-web`

> 📖 官方文档：<https://developers.weixin.qq.com/doc/oplatform/Website_App/WeChat_Login/Wechat_Login.html>

基于 OAuth2.0 协议，网站应用通过微信扫码完成登录。支持自动回调和手动调用两种方式。

### 授权流程

```
┌──────────┐     1. 跳转授权页         ┌──────────┐
│  前端页面 │ ──────────────────────▶ │  微信扫码 │
└──────────┘                          └──────────┘
     ▲                                    │
     │         2. 扫码授权, 回调 code       │
     │ ◀──────────────────────────────────┘
     │
     │  3. 后端通过 code 换取 access_token
     │  4. 获取用户信息
     │  5. 返回登录结果
     ▼
┌──────────┐
│  业务逻辑  │
└──────────┘
```

### 方式一：自动回调（推荐）

框架自动注册回调路由，用户扫码授权后微信重定向到回调地址，自动完成登录流程。

```rust
AFaster::new("config.toml".into())
    .await?
    .with_wxlogin(|w| {
        w.with_web_callback(|(state, result, oauth_state)| async move {
            println!("openid: {}", result.openid);
            Ok(result)
        })
    })
    .service(your_services)
    .run()
    .await;
```

前端跳转：

```rust
let url = state.wxlogin.get_authorize_url(Some("random_state"));
// => https://open.weixin.qq.com/connect/qrconnect?appid=...&redirect_uri=...&scope=snsapi_login&state=random_state#wechat_redirect
```

### 方式二：手动调用

```rust
// 1. 生成授权 URL
let authorize_url = state.wxlogin.get_authorize_url(Some("my_state"));

// 2. 用户扫码后，微信回调 redirect_uri?code=CODE&state=my_state

// 3. 通过 code 换取 access_token
let token = state.wxlogin.web_login("CODE").await?;

// 4. 获取用户信息
let userinfo = state.wxlogin.web_get_userinfo(&token.access_token, &token.openid, None).await?;

// 5. 刷新 access_token（有效期 2 小时，refresh_token 有效期 30 天）
let refreshed = state.wxlogin.web_refresh_token(&token.refresh_token).await?;

// 6. 检验 access_token 是否有效
let is_valid = state.wxlogin.web_check_token(&token.access_token, &token.openid).await?;

// 7. 完整登录流程（自动获取 userinfo）
let result = state.wxlogin.web_login_full("CODE").await?;
```

### 内嵌二维码

前端可使用微信 JS SDK 将二维码内嵌到页面中，用户扫码后通过 JS 获取 `code` 再调用后端接口：

```html
<script src="http://res.wx.qq.com/connect/zh_CN/htmledition/js/wxLogin.js"></script>
<script>
var obj = new WxLogin({
    self_redirect: true,
    id: "login_container",
    appid: "",
    scope: "snsapi_login",
    redirect_uri: encodeURIComponent("https://example.com/auth/wechat/callback"),
    state: "random_state",
    style: "black"
});
</script>
```

### API

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_authorize_url` | `state: Option<&str>` | `String` | 生成授权 URL |
| `web_login` | `code: &str` | `Result<WxWebAccessTokenResponse>` | 通过 code 换取 access_token |
| `web_refresh_token` | `refresh_token: &str` | `Result<WxWebAccessTokenResponse>` | 刷新 access_token |
| `web_check_token` | `access_token, openid` | `Result<bool>` | 检验 access_token 是否有效 |
| `web_get_userinfo` | `access_token, openid, lang` | `Result<WxWebUserInfo>` | 获取用户信息 |
| `web_login_full` | `code: &str` | `Result<WxWebLoginResult>` | 完整登录（token + userinfo） |

### WxWebAccessTokenResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `access_token` | `String` | 接口调用凭证 |
| `expires_in` | `i32` | 有效期（秒），7200 |
| `refresh_token` | `String` | 刷新凭证，有效期 30 天 |
| `openid` | `String` | 用户唯一标识 |
| `scope` | `String` | 授权作用域 |
| `unionid` | `Option<String>` | 统一标识 |

### WxWebUserInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| `openid` | `String` | 用户唯一标识 |
| `nickname` | `Option<String>` | 昵称 |
| `sex` | `Option<i32>` | 性别：1=男，2=女 |
| `province` | `Option<String>` | 省份 |
| `city` | `Option<String>` | 城市 |
| `country` | `Option<String>` | 国家 |
| `headimgurl` | `Option<String>` | 头像 URL |
| `privilege` | `Option<Vec<String>>` | 特权信息 |
| `unionid` | `Option<String>` | 统一标识 |

### WxWebLoginResult

| 字段 | 类型 | 说明 |
|------|------|------|
| `access_token` | `String` | 接口调用凭证 |
| `expires_in` | `i32` | 有效期（秒） |
| `refresh_token` | `String` | 刷新凭证 |
| `openid` | `String` | 用户唯一标识 |
| `scope` | `String` | 授权作用域 |
| `unionid` | `Option<String>` | 统一标识 |
| `userinfo` | `Option<WxWebUserInfo>` | 用户信息（scope 包含 snsapi_userinfo 时有值） |

---

## 错误码

### 小程序（模块 02）

| 码 | English | 中文 |
|----|---------|------|
| 40201 | WeChat API business error | 微信 API 业务错误 |
| 50201 | Mini login URL build failed | 构建请求 URL 失败 |
| 50202 | Mini login request failed | 发送请求失败 |
| 50203 | Mini login response parse failed | 解析响应 JSON 失败 |
| 50204 | Mini login response deserialize failed | 反序列化登录响应失败 |

### APP（模块 03）

| 码 | English | 中文 |
|----|---------|------|
| 40301 | WeChat API business error | 微信 API 业务错误 |
| 50301 | APP login URL build failed | 构建请求 URL 失败 |
| 50302 | APP login request failed | 发送请求失败 |
| 50303 | APP login response parse failed | 解析响应 JSON 失败 |
| 50304 | APP login response deserialize failed | 反序列化登录响应失败 |

### 网页扫码（模块 21）

| 码 | English | 中文 |
|----|---------|------|
| 42101 | WeChat API business error | 微信 API 业务错误 |
| 42102 | Missing code parameter | 缺少 code 参数 |
| 42103 | Missing state parameter | 缺少 state 参数 |
| 42104 | Invalid or expired state | state 验证失败 |
| 52101 | URL build failed | 构建请求 URL 失败 |
| 52102 | Access token request failed | 通过 code 换取 access_token 请求失败 |
| 52103 | Access token response parse failed | 解析 access_token 响应 JSON 失败 |
| 52104 | Access token deserialize failed | 反序列化 access_token 响应失败 |
| 52105 | Refresh token request failed | 刷新 access_token 请求失败 |
| 52106 | Refresh token response parse failed | 解析刷新 access_token 响应失败 |
| 52107 | Check token request failed | 检验 access_token 请求失败 |
| 52108 | Check token response parse failed | 解析检验 access_token 响应失败 |
| 52109 | Userinfo request failed | 获取用户信息请求失败 |
| 52110 | Userinfo response parse failed | 解析用户信息响应失败 |
| 52111 | Callback not registered | 回调函数未注册 |

## 参考文档

- [小程序登录](https://developers.weixin.qq.com/miniprogram/dev/api-backend/open-api/login/auth.code2Session.html)
- [APP 登录](https://developers.weixin.qq.com/doc/oplatform/Mobile_App/operation.html)
- [网站应用微信登录开发指南](https://developers.weixin.qq.com/doc/oplatform/Website_App/WeChat_Login/Wechat_Login.html)
- [授权后接口调用（UnionID）](https://developers.weixin.qq.com/doc/oplatform/Website_App/WeChat_Login/Authorized_Interface_Calling_UnionID.html)
