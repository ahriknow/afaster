# 推送服务

> Feature: `push` / `push-getui` / `push-jpush` / `push-xiaomi`

统一推送模块，支持多个第三方推送平台。根据启用的 feature 自动配置，无需手动注册。

## 配置

```toml
[push]

[push.getui]       # push-getui
app_id = ""
app_key = ""
master_secret = ""

[push.jpush]       # push-jpush
app_key = ""
master_secret = ""

[push.xiaomi]      # push-xiaomi
package_name = ""
app_secret = ""
```

启用对应 feature 后，配置为必填项，缺失则反序列化直接报错。

## 公共类型

```rust
use afaster::push::{PushTarget, Notification, PushSettings};

// 推送目标
let target = PushTarget::Cid("regid".to_string());   // 单个 CID/RegID
let target = PushTarget::Alias("user_1001".to_string()); // 别名
let target = PushTarget::Tag("vip".to_string());       // 标签
let target = PushTarget::All;                           // 全量

// 通知消息
let notification = Notification::new("标题", "内容")
    .url("https://example.com");

// 推送设置
let settings = PushSettings {
    ttl: Some(86400000),
    speed: Some(100),
    schedule_time: None,
};
```

## 各平台对比

| 特性 | 个推 | 极光 | 小米 |
|------|------|------|------|
| Feature | `push-getui` | `push-jpush` | `push-xiaomi` |
| 认证方式 | SHA256 签名 + token | HTTP Basic Auth | Authorization header |
| 通知推送 | ✅ | ✅ | ✅ |
| 透传消息 | ✅ | ✅ | ✅ |
| 批量推 | ✅ (toList) | ❌ | ❌ |
| 群推 | ✅ (toApp) | ✅ | ✅ |
| 标签推 | ✅ | ✅ | ✅ |
| Token 自动管理 | ✅ (缓存+被动刷新) | ❌ | ❌ |
| 厂商通道 | ✅ (自动配置) | ✅ (内置) | ✅ (原生) |

## 详细文档

- [个推推送](./getui.md)
- [极光推送](./jpush.md)
- [小米推送](./xiaomi.md)
