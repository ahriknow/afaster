# 个推推送

> Feature: `push-getui`

个推 REST API V2，支持单推/批量推/群推/条件筛选，厂商通道自动配置。

## 配置

```toml
[push.getui]
app_id = ""
app_key = ""
master_secret = ""
# base_url = "https://restapi.getui.com"
```

## API

### `send(target, notification, settings)` — 发送通知

```rust
let result = state.push.getui.send(
    &PushTarget::Cid("cid_value".to_string()),
    &Notification::new("标题", "内容").url("https://example.com"),
    None,
).await?;
```

### `send_transmission(target, content, settings)` — 发送透传

```rust
let result = state.push.getui.send_transmission(
    &PushTarget::Cid("cid_value".to_string()),
    "{\"action\":\"refresh\"}",
    None,
).await?;
```

### `create_list_message(notification, settings, group_name)` — 创建批量推消息

返回 `taskid`，用于 `send_list_cid` / `send_list_alias`。

### `send_list_cid(taskid, cids, is_async)` — 批量推 CID

### `send_list_alias(taskid, aliases, is_async)` — 批量推别名

## 推送目标

| PushTarget | 接口 | 说明 |
|-----------|------|------|
| `Cid` | `/push/single/cid` | 单个 CID |
| `Alias` | `/push/single/alias` | 单个别名 |
| `CidList` | `/push/single/batch/cid` | 批量单推（≤200） |
| `AliasList` | `/push/single/batch/alias` | 批量单推（≤200） |
| `Tag` | `/push/fast_custom_tag` | 标签快速推送 |
| `All` | `/push/all` | 全量推送 |

## Token 管理

- 自动缓存，提前 1 小时刷新
- 被动刷新：code=10001 时自动重试

## 参考文档

- [个推 REST API V2](https://docs.getui.com/getui/server/rest_v2/)
