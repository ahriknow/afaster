# 腾讯云短信

Feature: `sms-tencent`

> 📖 官方文档：<https://cloud.tencent.com/document/product/382/55981>

纯 Rust 实现，通过腾讯云 SMS API 直接发送短信，不依赖第三方 SDK。

## config.toml 配置

```toml
[sms_tencent]
secret_id = "your-secret-id"
secret_key = "your-secret-key"
sdk_app_id = "1400000000"    # 短信 SdkAppId
sign_name = "你的签名"        # 默认短信签名
template_id = "123456"        # 默认验证码模板 ID
```

## API

### SmsTencent

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `new` | - | `SmsTencent` | 创建空实例 |
| `send` | `phone_numbers[], sign_name?, template_id?, template_params?` | `Result<TencentSmsResponse>` | 发送短信（通用） |
| `send_code` | `phone_number, code, expire_minutes?` | `Result<TencentSmsResponse>` | 发送验证码短信 |
| `send_template` | `phone_numbers[], sign_name, template_id, template_params[]` | `Result<TencentSmsResponse>` | 发送自定义模板短信 |

### TencentSmsResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| `request_id` | `String` | 请求 ID |
| `send_status_set` | `Vec<SendStatus>` | 每个手机号的发送状态 |

### SendStatus

| 字段 | 类型 | 说明 |
|------|------|------|
| `serial_no` | `String` | 发送流水号 |
| `phone_number` | `String` | 手机号 |
| `fee` | `u32` | 计费条数 |
| `code` | `String` | 状态码，`"Ok"` 表示成功 |
| `message` | `String` | 状态描述 |
| `iso_code` | `String` | 国家码 |

## 使用示例

```rust
// 发送验证码（使用 config.toml 中的默认签名和模板）
let resp = state.sms_tencent.send_code("13800138000", "1234", Some("5")).await?;

// 发送自定义模板
let resp = state
    .sms_tencent
    .send_template(
        &["13800138000"],
        "你的签名",
        "123456",
        vec!["张三", "20250101"],
    )
    .await?;

// 批量发送
let resp = state
    .sms_tencent
    .send(
        &["13800138000", "13900139000"],
        None,
        None,
        Some(vec!["5678"]),
    )
    .await?;
```

## 签名机制

使用腾讯云 TC3-HMAC-SHA256 签名，纯 Rust 实现。与 COS 模块使用相同的签名模式。

## 回执回调

启用 `sms-tencent` feature 后，框架会自动在 `config.toml` 中 `sms_tencent.callback_path`（默认 `sms/tencent/report`）注册 POST 路由，用于接收腾讯云推送的短信回执报告。

### 配置

```toml
[sms_tencent]
callback_path = "sms/tencent/report"  # 可选，默认值如左
```

### 注册回调

```rust
use afaster::SmsTencent;
use afaster::state::smstencent::{TencentSmsReport, TencentSmsCallbackResult};

let app = AFaster::new()
    .with_sms_tencent(|tencent| {
        tencent.with_report_callback(|(state, reports): (AppState, Vec<TencentSmsReport>)| {
            async move {
                for report in reports {
                    if report.report_status == "SUCCESS" {
                        tracing::info!(phone = %report.mobile, "短信送达");
                    } else {
                        tracing::warn!(phone = %report.mobile, err = %report.errmsg, "短信未送达");
                    }
                }
                Ok(TencentSmsCallbackResult::ok())
            }
        })
    })
    .run()
    .await;
```

> **注意**：未注册回调时，回执报告会被静默丢弃并返回成功（避免云平台重试），开启 `log` feature 会打印 debug 日志。

### TencentSmsReport 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `user_receive_time` | `String` | 用户实际接收时间 |
| `nationcode` | `String` | 国家码 |
| `mobile` | `String` | 手机号 |
| `report_status` | `String` | 送达状态：`SUCCESS` / `FAIL` |
| `errmsg` | `String` | 错误信息 |
| `description` | `String` | 状态描述 |
| `sid` | `String` | 发送标识 ID |
| `ext` | `String` | 用户 session 内容 |

## 错误码

| 错误码 | English | 中文 |
|--------|---------|------|
| 41101 | SMS credentials not configured | 短信凭证未配置 |
| 51102 | HMAC-SHA256 init failed | HMAC-SHA256 初始化失败 |
| 51103 | Tencent SMS request failed | 腾讯云短信请求失败 |
| 51104 | Tencent SMS response parse failed | 腾讯云短信响应解析失败 |
| 51105 | Tencent SMS API error | 腾讯云短信 API 返回错误 |
| 51107 | SMS report callback not registered | 短信回执回调未注册 |
