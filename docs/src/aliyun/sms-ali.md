# 阿里云短信

Feature: `sms-ali`

> 📖 官方文档：<https://help.aliyun.com/zh/sms/developer-reference/api-dysmsapi-2017-05-25-sendsms>

纯 Rust 实现，通过阿里云 SMS API 直接发送短信，不依赖第三方 SDK。

## config.toml 配置

```toml
[sms_ali]
access_key_id = "your-access-key-id"
access_key_secret = "your-access-key-secret"
sign_name = "你的签名"       # 默认短信签名
template_code = "SMS_123456" # 默认验证码模板 Code
```

## API

### SmsAli

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `new` | - | `SmsAli` | 创建空实例 |
| `send` | `phone_numbers, sign_name?, template_code?, template_param?` | `Result<AliSmsResponse>` | 发送短信（通用） |
| `send_code` | `phone_numbers, code` | `Result<AliSmsResponse>` | 发送验证码短信 |
| `send_template` | `phone_numbers, sign_name, template_code, template_param` | `Result<AliSmsResponse>` | 发送自定义模板短信 |
| `with_report_callback` | `callback` | `Self` | 注册短信回执回调函数 |

### AliSmsResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `request_id` | `String` | 请求 ID |
| `code` | `String` | 状态码，`"OK"` 表示成功 |
| `message` | `String` | 状态描述 |
| `biz_id` | `Option<String>` | 发送回执 ID |

## 使用示例

```rust
// 发送验证码（使用 config.toml 中的默认签名和模板）
let resp = state.sms_ali.send_code("13800138000", "1234").await?;

// 发送自定义模板
let resp = state
    .sms_ali
    .send_template(
        "13800138000",
        "你的签名",
        "SMS_123456",
        r#"{"name":"张三","order":"20250101"}"#,
    )
    .await?;

// 批量发送（逗号分隔，最多 100 个）
let resp = state
    .sms_ali
    .send("13800138000,13900139000", None, None, Some(r#"{"code":"5678"}"#))
    .await?;
```

## 签名机制

使用阿里云 RPC 签名 V1（HMAC-SHA1），与 OSS 模块相同的签名方式，纯 Rust 实现。

## 回执回调

启用 `sms-ali` feature 后，框架会自动在 `config.toml` 中 `sms_ali.callback_path`（默认 `sms/ali/report`）注册 POST 路由，用于接收阿里云推送的短信回执报告。

### 配置

```toml
[sms_ali]
callback_path = "sms/ali/report"  # 可选，默认值如左
```

### 注册回调

```rust
use afaster::SmsAli;
use afaster::state::smsali::{AliSmsReport, AliSmsCallbackResult};

let app = AFaster::new()
    .with_sms_ali(|ali| {
        ali.with_report_callback(|(state, reports): (AppState, Vec<AliSmsReport>)| {
            async move {
                for report in reports {
                    if report.success {
                        tracing::info!(phone = %report.phone_number, "短信送达");
                    } else {
                        tracing::warn!(phone = %report.phone_number, err = %report.err_code, "短信未送达");
                    }
                }
                Ok(AliSmsCallbackResult::ok())
            }
        })
    })
    .run()
    .await;
```

> **注意**：未注册回调时，回执报告会被静默丢弃并返回成功（避免云平台重试），开启 `log` feature 会打印 debug 日志。

### AliSmsReport 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `biz_id` | `String` | 发送回执 ID（与 SendSms 返回的 BizId 对应） |
| `phone_number` | `String` | 手机号 |
| `send_time` | `String` | 发送时间 |
| `report_time` | `String` | 运营商回执时间 |
| `success` | `bool` | 是否成功送达 |
| `err_code` | `String` | 运营商错误码（成功时为空） |
| `err_msg` | `String` | 运营商错误描述 |
| `template_code` | `String` | 短信模板 Code |
| `sign_name` | `String` | 短信签名 |
| `dest_code` | `String` | 上行短信扩展码（SmsUp 类型） |
| `content` | `String` | 上行短信内容（SmsUp 类型） |

## 错误码

| 错误码 | English | 中文 |
|--------|---------|------|
| 41101 | SMS credentials not configured | 短信凭证未配置 |
| 51101 | HMAC-SHA1 init failed | HMAC-SHA1 初始化失败 |
| 51103 | Alibaba SMS request failed | 阿里云短信请求失败 |
| 51104 | Alibaba SMS response parse failed | 阿里云短信响应解析失败 |
| 51105 | Alibaba SMS API error | 阿里云短信 API 返回错误 |
| 51106 | SMS report callback not registered | 短信回执回调未注册 |
