# 限流模块 (rate-limit)

## 简介

基于 afast 框架内置限流方案的配置模块，支持固定窗口、滑动窗口和令牌桶三种算法，按 IP/Header/连接/全局等维度进行限流。

## Feature

```toml
afaster = { version = "0.0.7", default-features = false, features = ["rate-limit"] }
```

## 配置

在 `config.toml` 中添加：

```toml
[rate_limit]
enabled = true                # 是否启用限流, 默认 true
# default_policy = "global"   # 默认策略 ID（未在 handler 上指定时使用）
rejected_code = 42001         # 被拒绝时的错误码
rejected_message = "Too many requests"

# 全局限流策略
[[rate_limit.policies]]
id = "global"
max_requests = 100
window_seconds = 60
algorithm = "token_bucket"    # fixed_window / sliding_window / token_bucket
key = "ip"                    # ip / header / connection / global

# 登录防爆破
[[rate_limit.policies]]
id = "login"
max_requests = 5
window_seconds = 60
algorithm = "sliding_window"
key = "ip"
```

## 使用方法

### 1. 配置 config.toml

参考上方配置段，定义限流策略。

### 2. 在 Handler 上声明限流策略

```rust
use afast::{handler, Data, State, Result};

#[handler(desc("登录接口"), rate_limit("login"))]
async fn login(
    state: State<AppState>,
    req: Data<LoginReq>,
) -> Result<LoginResp> {
    // 框架自动按 "login" 策略限流
    // ...
}
```

### 3. 启动应用

`AFaster::run()` 会自动从 `config.toml` 读取限流配置并注册到 afast，无需手动操作：

```rust
use afaster::AFaster;

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string())
        .await
        .unwrap()
        .run()
        .await;
}
```

## 算法说明

| 算法 | 说明 | 适用场景 |
|------|------|----------|
| `fixed_window` | 固定时间窗口，窗口边界重置 | 一般限流 |
| `sliding_window` | 滑动窗口，避免边界突发 | 登录防爆破、安全场景 |
| `token_bucket` | 令牌桶，允许短时突发 | API 限流、一般业务 |

## Key 提取方式

| 方式 | 说明 |
|------|------|
| `ip` | 按客户端 IP 限流 |
| `header` | 按 HTTP Header 值限流（需配置 `header_name`） |
| `connection` | 按连接限流（WS/TCP 消息频率） |
| `global` | 全局共享计数器 |

## 错误码

| 错误码 | 说明 |
|--------|------|
| 42001 | 请求频率超限（默认） |

## 注意事项

1. `AFaster::run()` 会自动注册限流配置，无需手动调用 `rate_limit()`
2. handler 通过 `rate_limit("policy_id")` 属性引用策略
3. 如需分布式限流，可实现 `RateLimitStore` trait 接入 Redis
4. `key = "header"` 时需额外配置 `header_name` 字段
