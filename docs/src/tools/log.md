# 日志

Feature: `log`

## 依赖

```toml
log = ["tracing", "tracing-appender", "tracing-subscriber"]
```

## 日志文件

| 文件 | 级别 | 格式 |
|------|------|------|
| `logs/error.log.YYYY-MM-DD` | ERROR | JSON |
| `logs/warn.log.YYYY-MM-DD` | WARN | JSON |
| `logs/info.log.YYYY-MM-DD` | INFO | JSON |

标准输出: DEBUG 及以上级别

## 宏

通过 `lib.rs` 导出:

```rust
pub use tracing::{debug, error, info, trace, warn};
```

## 使用示例

```rust
use afaster::{debug, error, info, warn};

info!("Server started");
warn!("Cache miss: {}", key);
error!({ code = 50002, msg = "数据库错误" }, "Connection failed: {}", err);
debug!("Request processed in {:?}", duration);
```

## 带上下文的日志

```rust
tracing::error!(
    { code = 50201, msg = "微信小程序登录错误" },
    "详细错误信息: {}", e
);
```
