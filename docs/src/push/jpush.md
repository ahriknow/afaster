# 极光推送

> Feature: `push-jpush`

极光 JPush REST API V3，支持通知/透传/多平台，HTTP Basic Auth。

## 配置

```toml
[push.jpush]
app_key = ""
master_secret = ""
# base_url = "https://api.jpush.cn"
# apns_production = true
```

## API

### `send(target, notification, options)` — 发送通知

```rust
let result = state.push.jpush.send(
    &PushTarget::Cid("registration_id".to_string()),
    &Notification::new("标题", "内容"),
    None,
).await?;
```

### `send_message(target, content, content_type, extras, options)` — 发送透传

```rust
let result = state.push.jpush.send_message(
    &PushTarget::All,
    "{\"action\":\"refresh\"}",
    Some("text"),
    None,
    None,
).await?;
```

### `send_notification_and_message(target, notification, content, options)` — 同时发送

## 推送目标

| PushTarget | audience | 说明 |
|-----------|----------|------|
| `Cid` | `registration_id` | 单个注册 ID |
| `CidList` | `registration_id` | 多个注册 ID |
| `Alias` | `alias` | 单个别名 |
| `AliasList` | `alias` | 多个别名 |
| `Tag` | `tag` | 标签（OR） |
| `All` | `"all"` | 广播 |

## 参考文档

- [极光推送 API V3](https://docs.jiguang.cn/jpush/server/push/rest_api_v3_push)
