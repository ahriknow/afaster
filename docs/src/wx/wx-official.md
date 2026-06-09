# 微信公众号管理

> Feature: `wx-official`

微信公众号管理模块，提供模板消息、自定义菜单、素材管理等能力，内置 access_token 自动缓存和刷新。

## 配置

```toml
[wx_official]
app_id = "wx1234567890"
app_secret = "your-app-secret"
# base_url = "https://api.weixin.qq.com"  # 默认值
```

## access_token 管理

模块自动管理 access_token 的获取和缓存，有效期 2 小时，提前 5 分钟自动刷新。使用稳定版接口 `/cgi-bin/stable_token`。

```rust
// 获取 access_token（自动缓存）
let token = state.wx_official.get_access_token().await?;

// 强制刷新
let token = state.wx_official.refresh_access_token(true).await?;
```

## 模板消息

```rust
use afaster::wx_official::TemplateDataBuilder;

let data = TemplateDataBuilder::new()
    .value("name01", "张三")
    .value("amount01", "￥100")
    .value("thing01", "广州至北京")
    .value("date01", "2024-01-01")
    .build();

// 发送模板消息
let msgid = state.wx_official.send_template_message(
    "openid",                    // 接收者
    "template_id",               // 模板 ID
    data,                        // 模板数据
    Some("https://example.com"), // 跳转链接 (可选)
    None,                        // 跳转小程序 (可选)
).await?;
```

## 自定义菜单

```rust
let buttons = serde_json::json!([
    {
        "type": "click",
        "name": "今日歌曲",
        "key": "V1001_TODAY_MUSIC"
    },
    {
        "name": "菜单",
        "sub_button": [
            { "type": "view", "name": "搜索", "url": "http://www.soso.com/" },
            { "type": "click", "name": "赞一下", "key": "V1001_GOOD" }
        ]
    }
]);

// 创建菜单
state.wx_official.create_menu(buttons).await?;

// 获取菜单
let menu = state.wx_official.get_menu().await?;

// 查询当前菜单（含官网设置的）
let info = state.wx_official.get_current_selfmenu_info().await?;

// 删除菜单
state.wx_official.delete_menu().await?;

// 创建个性化菜单
let menuid = state.wx_official.add_conditional_menu(
    buttons,
    serde_json::json!({ "tag_id": "2" }),
).await?;

// 删除个性化菜单
state.wx_official.delete_conditional_menu(&menuid).await?;

// 测试匹配
let matched = state.wx_official.trymatch_menu("openid").await?;
```

## 素材管理

```rust
// 获取永久素材总数
let count = state.wx_official.get_material_count().await?;
println!("图片: {}, 语音: {}, 视频: {}, 图文: {}", 
    count.image_count, count.voice_count, count.video_count, count.news_count);

// 获取永久素材列表
let list = state.wx_official.batchget_material("image", 0, 20).await?;
for item in &list.item {
    println!("{}: {} ({})", item.media_id, item.name.as_deref().unwrap_or(""), item.url.as_deref().unwrap_or(""));
}

// 删除永久素材
state.wx_official.delete_material("MEDIA_ID").await?;
```

## API

| 方法 | 说明 |
|------|------|
| `get_access_token()` | 获取 access_token（自动缓存） |
| `refresh_access_token(force)` | 刷新 access_token |
| `send_template_message(...)` | 发送模板消息 |
| `create_menu(buttons)` | 创建自定义菜单 |
| `get_menu()` | 获取菜单配置 |
| `get_current_selfmenu_info()` | 查询当前菜单（含官网） |
| `delete_menu()` | 删除菜单 |
| `add_conditional_menu(buttons, matchrule)` | 创建个性化菜单 |
| `delete_conditional_menu(menuid)` | 删除个性化菜单 |
| `trymatch_menu(user_id)` | 测试个性化菜单匹配 |
| `get_material_count()` | 获取永久素材总数 |
| `batchget_material(type, offset, count)` | 获取永久素材列表 |
| `delete_material(media_id)` | 删除永久素材 |

## 参考文档

- [模板消息](https://developers.weixin.qq.com/doc/offiaccount/Message_Management/Template_Message_Interface.html)
- [自定义菜单](https://developers.weixin.qq.com/doc/offiaccount/Custom_Menus/Creating_Custom-Defined_Menu.html)
- [素材管理](https://developers.weixin.qq.com/doc/offiaccount/Asset_Management/Adding_Permanent_Assets.html)
- [access_token](https://developers.weixin.qq.com/doc/offiaccount/Basic_Information/Get_access_token.html)
- [公众号网页授权](https://developers.weixin.qq.com/doc/service/guide/h5/auth.html)

---

## 公众号网页授权登录

公众号服务号在**微信内置浏览器**中通过 OAuth2.0 网页授权获取用户信息。

### 配置

```toml
[wx_official]
app_id = "wx1234567890"
app_secret = "your-app-secret"
redirect_base = "https://example.com"   # 授权回调基础地址
# callback_path = "auth/wechat/mp/callback"  # 默认值
```

### Scope 说明

| Scope | 说明 |
|-------|------|
| `snsapi_base` | 静默授权，不弹窗，仅获取 openid |
| `snsapi_userinfo` | 弹窗授权，可获取昵称、头像等用户信息 |

### 方式一：自动回调（推荐）

```rust
AFaster::new("config.toml".into())
    .await?
    .with_wx_official(|w| {
        w.with_mp_callback(|(state, result, oauth_state)| async move {
            println!("openid: {}", result.openid);
            Ok(result)
        })
    })
    .service(your_services)
    .run()
    .await;
```

前端（微信内置浏览器中）跳转：

```rust
let url = state.wx_official.mp_get_authorize_url("snsapi_userinfo", Some("state"));
```

### 方式二：手动调用

```rust
// 1. 生成授权 URL
let authorize_url = state.wx_official.mp_get_authorize_url("snsapi_userinfo", Some("my_state"));

// 2. 通过 code 换取 access_token
let token = state.wx_official.mp_login("CODE").await?;

// 3. 获取用户信息
let userinfo = state.wx_official.mp_get_userinfo(&token.access_token, &token.openid, None).await?;

// 4. 刷新 access_token
let refreshed = state.wx_official.mp_refresh_token(&token.refresh_token).await?;

// 5. 检验 access_token 是否有效
let is_valid = state.wx_official.mp_check_token(&token.access_token, &token.openid).await?;

// 6. 完整登录流程
let result = state.wx_official.mp_login_full("CODE").await?;
```

### 网页授权 API

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `mp_get_authorize_url` | `scope, state` | `String` | 生成授权 URL |
| `mp_login` | `code` | `Result<WxAccessTokenResponse>` | 通过 code 换取 access_token |
| `mp_refresh_token` | `refresh_token` | `Result<WxAccessTokenResponse>` | 刷新 access_token |
| `mp_check_token` | `access_token, openid` | `Result<bool>` | 检验 access_token 是否有效 |
| `mp_get_userinfo` | `access_token, openid, lang` | `Result<WxUserInfo>` | 获取用户信息 |
| `mp_login_full` | `code` | `Result<WxMpLoginResult>` | 完整登录（token + userinfo） |

### 类型定义

#### WxAccessTokenResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `access_token` | `String` | 接口调用凭证 |
| `expires_in` | `i32` | 有效期（秒），7200 |
| `refresh_token` | `String` | 刷新凭证，有效期 30 天 |
| `openid` | `String` | 用户唯一标识 |
| `scope` | `String` | 授权作用域 |
| `unionid` | `Option<String>` | 统一标识 |

#### WxUserInfo

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

#### WxMpLoginResult

| 字段 | 类型 | 说明 |
|------|------|------|
| `access_token` | `String` | 接口调用凭证 |
| `expires_in` | `i32` | 有效期（秒） |
| `refresh_token` | `String` | 刷新凭证 |
| `openid` | `String` | 用户唯一标识 |
| `scope` | `String` | 授权作用域 |
| `unionid` | `Option<String>` | 统一标识 |
| `userinfo` | `Option<WxUserInfo>` | 用户信息（scope 包含 snsapi_userinfo 时有值） |
