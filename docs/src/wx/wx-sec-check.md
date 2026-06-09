# 微信内容安全

Feature: `wx-sec-check`

> 📖 官方文档：<https://developers.weixin.qq.com/miniprogram/dev/api-backend/open-api/sec-check/security.msgSecCheck.html>

## 配置

```toml
[wx_sec_check]
app_id     = "wx1234567890"
app_secret = "your-secret"
# base_url = "https://api.weixin.qq.com"  # 可选，默认值
```

## API

### WxSecCheck

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `msg_sec_check` | `&MsgSecCheckRequest` | `Result<MsgSecCheckResponse>` | 文本内容安全检测 |
| `media_check_async` | `&MediaCheckRequest` | `Result<MediaCheckResponse>` | 异步媒体内容安全检测（结果由微信推送） |

### MsgSecCheckRequest

```rust
// Builder 模式构造
let request = MsgSecCheckRequest::new(
    "待检测文本",       // content: &str
    Scene::Comment,     // scene: Scene
    "user_openid",      // openid: &str
)
.title("标题")         // 可选
.nickname("昵称")      // 可选
.signature("签名");    // 可选，仅 scene=Profile 有效
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| content | String | ✅ | 文本内容，上限 2500 字，UTF-8 |
| version | u8 | ✅ | 固定值 2（自动设置） |
| scene | u8 | ✅ | 场景：1 资料 / 2 评论 / 3 论坛 / 4 社交日志 |
| openid | String | ✅ | 用户 openid |
| title | Option\<String\> | ❌ | 文本标题 |
| nickname | Option\<String\> | ❌ | 用户昵称 |
| signature | Option\<String\> | ❌ | 个性签名（仅 scene=1） |

### MsgSecCheckResponse

```rust
pub struct MsgSecCheckResponse {
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
    pub trace_id: Option<String>,
    pub result: Option<SecCheckResult>,     // 综合结果
    pub detail: Option<Vec<SecCheckDetail>>,// 详细检测结果
}
```

### MediaCheckRequest

```rust
let request = MediaCheckRequest::new(
    "https://example.com/image.jpg",  // media_url
    MediaType::Image,                  // media_type: 1=音频, 2=图片
    Scene::Comment,                    // scene
    "user_openid",                     // openid
);
```

> 异步检测结果会在 30 分钟内推送到消息接收服务器，推送事件为 `wxa_media_check`。
> 文件大小限制 10MB。

### 场景枚举 (Scene)

| 值 | 名称 | 说明 |
|----|------|------|
| 1 | Profile | 资料 |
| 2 | Comment | 评论 |
| 3 | Forum | 论坛 |
| 4 | Social | 社交日志 |

### 建议枚举 (Suggest)

| 值 | 说明 |
|----|------|
| Pass | 通过 |
| Review | 需人工审核 |
| Risky | 拦截 |

### 标签枚举 (Label)

| 值 | 名称 | 说明 |
|----|------|------|
| 100 | Normal | 正常 |
| 10001 | Ad | 广告 |
| 20001 | Politics | 时政 |
| 20002 | Porn | 色情 |
| 20003 | Abuse | 辱骂 |
| 20006 | Illegal | 违法犯罪 |
| 20008 | Fraud | 欺诈 |
| 20012 | Vulgar | 低俗 |
| 20013 | Copyright | 版权 |
| 21000 | Other | 其他 |

### 媒体类型 (MediaType)

| 值 | 名称 | 支持格式 |
|----|------|----------|
| 1 | Audio | mp3, aac, ac3, wma, flac, vorbis, opus, wav |
| 2 | Image | jpg, jpeg, png, bmp, gif (取首帧) |

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 40701 | WeChat API business error | 微信 API 业务错误 |
| 40702 | Content is empty or exceeds 2500 characters | 内容为空或超过 2500 字 |
| 50701 | Content check URL build failed | 构建请求 URL 失败 |
| 50702 | Content check request failed | 发送请求失败 |
| 50703 | Content check response parse failed | 解析响应 JSON 失败 |
| 50704 | Content check response deserialize failed | 反序列化响应失败 |
| 50705 | Failed to get access_token | 获取 access_token 失败 |
| 50706 | Access_token response parse failed | access_token 响应解析失败 |
| 50806 | Media detection notification parse error | 内容安全通知解析失败 |
| 50815 | Media detection callback not registered | 未注册媒体检测回调 |

## 使用示例

### 文本检测

```rust
use afaster::state::wx_sec_check::{MsgSecCheckRequest, Scene};

let request = MsgSecCheckRequest::new(
    "待检测的文本内容",
    Scene::Comment,
    "user_openid",
);

let response = state.wx_sec_check.msg_sec_check(&request).await?;

if let Some(result) = &response.result {
    match result.suggest.as_str() {
        "risky" => println!("拦截: label={}", result.label),
        "review" => println!("需审核: label={}", result.label),
        "pass" => println!("通过"),
        _ => {}
    }
}
```

### 媒体检测

```rust
use afaster::state::wx_sec_check::{MediaCheckRequest, MediaType, Scene};

let request = MediaCheckRequest::new(
    "https://example.com/image.jpg",
    MediaType::Image,
    Scene::Comment,
    "user_openid",
);

let response = state.wx_sec_check.media_check_async(&request).await?;
println!("trace_id: {:?}", response.trace_id);
```

### 注册媒体检测回调

> **统一通知模块**：媒体检测结果由 `wx-notify` 模块统一接收和分发。
> 微信后台只需配置一个回调 URL（默认 `https://your-domain/wx/notify`），
> 后端通过 `Event` 字段自动路由到 `wxa_media_check` 回调。
>
> 详细配置参见 `config.toml` 中的 `[wx_notify]` 段。

```rust
use afaster::AFaster;

AFaster::new()
    .with_wx_notify(|w| {
        w.with_media_check_callback(|(state, notify)| {
            async move {
                // 处理异步检测结果
                if let Some(result) = &notify.result {
                    println!("检测结果: {:?}", result.suggest);
                }
                if let Some(detail) = &notify.detail {
                    for item in detail {
                        println!("策略: {}, 建议: {}, 标签: {}", item.strategy, item.suggest, item.label);
                    }
                }
                Ok(WxSecCheckNotifyResult::success())
            }
        })
    })
    .run()
    .await;
```

> **注意**：未注册回调时，`wx-notify` 模块返回错误码 50815，微信会重试最多 15 次。
