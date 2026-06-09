# 小米推送

> Feature: `push-xiaomi`

小米推送 V3 API，支持通知/透传/regid/别名/标签/广播。

## 配置

```toml
[push.xiaomi]
package_name = ""
app_secret = ""
# base_url = "https://api.xmpush.xiaomi.com"
```

## API

### `send(target, notification, options)` — 发送通知

```rust
let result = state.push.xiaomi.send(
    &PushTarget::Cid("registration_id".to_string()),
    &Notification::new("标题", "内容"),
    None,
).await?;
```

### `send_transmission(target, content, options)` — 发送透传

```rust
let result = state.push.xiaomi.send_transmission(
    &PushTarget::Cid("registration_id".to_string()),
    "{\"action\":\"refresh\"}",
    None,
).await?;
```

## 推送目标

| PushTarget | 接口 | 说明 |
|-----------|------|------|
| `Cid` | `/v3/message/regid` | 单个 regid |
| `CidList` | `/v3/message/regid` | 多个 regid（逗号分隔） |
| `Alias` | `/v3/message/alias` | 单个别名 |
| `AliasList` | `/v3/message/alias` | 多个别名 |
| `Tag` | `/v3/message/topic` | 标签 |
| `All` | `/v3/message/all` | 广播 |

## 推送选项

| 选项 | 说明 |
|------|------|
| `time_to_live` | 消息有效期（毫秒） |
| `time_to_send` | 定时发送（毫秒时间戳） |
| `notify_id` | 通知栏 ID（覆盖） |
| `notify_type` | 1=声音, 2=震动, 3=声音+震动, 4=静默 |
| `channel_id` | Android 通知渠道 |
| `notify_effect` | 1=打开首页, 2=打开Activity, 3=打开网页 |
| `web_uri` | 打开网页（notify_effect=3） |
| `intent_uri` | 打开 Activity（notify_effect=2） |
| `sound_uri` | 自定义铃声 |
| `notify_foreground` | 前台是否弹出 |
| `flow_control` | 平滑推送速度 |
| `jobkey` | 消息去重 key |

## 参考文档

- [小米推送服务端 API](https://dev.mi.com/xiaomihyperos/documentation/detail?pId=1559)
