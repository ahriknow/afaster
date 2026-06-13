# AFaster

基于 [afast](https://crates.io/crates/afast) 的高性能 Rust 后端框架模板，内置常用业务模块，开箱即用。

## 快速开始

```bash
cargo run
```

```rust
use afaster::AFaster;

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .run()
        .await;
}
```

### 静态文件服务（Vue / React SPA）

```rust
use afaster::{AFaster, serve::Serve};

#[tokio::main]
async fn main() {
    // 运行时目录模式
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .with_serve(Serve::from_dir("./dist").with_spa(true))
        .run()
        .await;
}
```

```rust
// 编译期嵌入模式（整个 dist 目录打包进二进制）
AFaster::new("config.toml".to_string()).await
    .unwrap()
    .with_serve(Serve::from_embedded(include_dir!("$CARGO_MANIFEST_DIR/dist")).with_spa(true))
    .run()
    .await;
```

## 功能模块

所有功能通过 Cargo feature 控制，按需启用。详见 [config.toml](config.toml)。

### 认证与安全

| Feature | 说明 |
|---------|------|
| `jwt` | JWT 令牌认证 |
| `nonce` | 随机字符串（OAuth2 state、CSRF） |
| `github-oauth2` | GitHub OAuth2 登录 |
| `argon2-hash` | Argon2 密码哈希 |
| `rbac` | RBAC 权限管理 |
| `rate-limit` | 令牌桶/滑动窗口限流 |

### 微信生态

| Feature | 说明 |
|---------|------|
| `wx-login-mini` | 微信小程序登录 |
| `wx-login-app` | 微信 APP 登录 |
| `wx-login-web` | 微信网页扫码登录 |
| `wx-official` | 微信公众号管理（模板消息、菜单、素材、网页授权） |
| `wx-pay-h5` | 微信 H5 支付 |
| `wx-pay-native` | 微信 Native 支付 |
| `wx-pay-app` | 微信 APP 支付 |
| `wx-pay-mini` | 微信小程序支付 |
| `wx-pay-js` | 微信 JSAPI 支付 |
| `wx-virtual-pay` | 微信虚拟支付 |
| `wx-sec-check` | 微信内容安全检测 |

### 阿里云

| Feature | 说明 |
|---------|------|
| `ali-pay-web` | 支付宝电脑网站支付 |
| `oss` | 阿里云 OSS 对象存储 |
| `sms-ali` | 阿里云短信 |

### 腾讯云

| Feature | 说明 |
|---------|------|
| `cos` | 腾讯云 COS 对象存储 |
| `sms-tencent` | 腾讯云短信 |
| `tmap` | 腾讯地图 |

### 数据与缓存

| Feature | 说明 |
|---------|------|
| `db-postgres` | PostgreSQL |
| `db-sqlite` | SQLite |
| `db-mysql` | MySQL |
| `redis` | Redis |
| `valkey` | Valkey（与 Redis 共享实现） |
| `memkv` | 内存 KV 数据库 |
| `scheduler` | 定时任务调度器 |

### 文件处理

| Feature | 说明 |
|---------|------|
| `file` | 本地文件服务 |
| `serve` | 静态网页服务（Vue/React SPA，支持运行时目录 / 编译期嵌入） |
| `serve-embed` | 编译期嵌入整个目录到二进制文件 |
| `image` | 图片生成 |
| `excel` | Excel / CSV 导入导出 |
| `pdf` | PDF 生成 |

### 消息通知

| Feature | 说明 |
|---------|------|
| `email` | 邮件发送（SMTP） |
| `push` | 推送服务（个推/极光/小米） |
| `sse` | Server-Sent Events |

### 地图

| Feature | 说明 |
|---------|------|
| `amap` | 高德地图 |

### 工具

| Feature | 说明 |
|---------|------|
| `log` | 日志（tracing） |
| `trace` | 链路追踪（可选后端：`trace-sqlite` / `trace-http` / `trace-tcp`） |
| `snow` | Snowflake ID 生成器 |
| `clock` | 时钟工具 |
| `regex-util` | 正则工具 |

> **提示**：`db-postgres`、`db-sqlite`、`db-mysql` 可同时启用。单数据库时可用 `state.db.pool()`，多数据库时用 `state.db.pg()` / `state.db.sqlite()` / `state.db.mysql()`。

## 配置

所有模块统一通过 `config.toml` 配置。完整示例见 [config.toml](config.toml)。

### 命名规范

- `set_*` — 设置实例（如 `set_rbac()`、`set_trace_store()`）
- `with_*` — 配置模块（闭包方式，如 `with_wx_pay(|w| w...)`）
- `with_*_callback` — 注册回调（如 `with_github_callback(cb)`）

### 回调机制

支持回调的模块通过链式方法注册闭包：

```rust
use afaster::AFaster;

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .with_github_oauth2(|g| g.with_github_callback(|(state, result, oauth_state)| {
            async move {
                // 处理 GitHub OAuth2 回调
                Ok(result)
            }
        }))
        .with_wx_virtual_pay(|w| w
            .with_goods_deliver_callback(|(state, notify)| {
                async move {
                    // 处理微信虚拟支付道具发货
                    Ok(afaster::wx_virtual_pay::WxPayNotifyResult::success())
                }
            })
        )
        .run()
        .await;
}
```

支持的回调方法：

| 模块 | 方法 | 说明 |
|------|------|------|
| `github-oauth2` | `with_github_oauth2(\|g\| g.with_github_callback(cb))` | GitHub OAuth2 登录回调 |
| `wx-login-web` | `with_wxlogin(\|w\| w.with_web_callback(cb))` | 微信网页扫码登录回调 |
| `wx-official` | `with_wx_official(\|w\| w.with_mp_callback(cb))` | 公众号网页授权登录回调 |
| `wx-virtual-pay` | `with_wx_virtual_pay(\|w\| w.with_goods_deliver_callback(cb))` | 道具发货回调 |
| `wx-virtual-pay` | `with_wx_virtual_pay(\|w\| w.with_coin_pay_callback(cb))` | 代币支付回调 |
| `wx-virtual-pay` | `with_wx_virtual_pay(\|w\| w.with_refund_callback(cb))` | 退款回调 |
| `wx-virtual-pay` | `with_wx_virtual_pay(\|w\| w.with_complaint_callback(cb))` | 用户投诉回调 |
| `ali-pay-web` | `with_ali_pay(\|a\| a.with_pay_success_callback(cb))` | 支付宝支付成功回调 |
| `ali-pay-web` | `with_ali_pay(\|a\| a.with_refund_callback(cb))` | 支付宝退款回调 |
| `sms-ali` | `with_sms_ali(\|a\| a.with_report_callback(cb))` | 阿里云短信回执回调 |
| `sms-tencent` | `with_sms_tencent(\|t\| t.with_report_callback(cb))` | 腾讯云短信回执回调 |

## 错误码

错误码为 5 位数字：第 1 位表示类别（`4` = 用户错误，`5` = 内部错误），第 2~3 位表示模块编号，第 4~5 位表示模块内序号。

完整错误码列表见 [docs/src/errors.md](docs/src/errors.md)。

## 文档

完整文档见 [docs/](docs/)，使用 mdBook 构建：

```bash
mdbook serve docs -p 5500
```

## 环境要求

- Rust 2024 edition
- Tokio async runtime

## License

MIT
