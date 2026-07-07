# 快速开始

## 安装

```toml
[dependencies]
afaster = { version = "0.0.6", features = ["jwt", "log", "redis"] }
```

## 配置

```toml
# config.toml
[backend]
host = "0.0.0.0"
port = 5000

[redis]
host = "127.0.0.1"
port = 6379
```

## 启动

```rust
use afaster::AFaster;

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string())
        .await
        .expect("初始化失败")
        .run()
        .await;
}
```

## Feature 列表

所有功能通过 Cargo feature 控制，按需启用。详见 [config.toml](https://github.com/ahriknow/afaster/blob/develop/config.toml)。

### 认证与安全

| Feature | 说明 |
|---------|------|
| `jwt` | JWT 令牌认证 |
| `nonce` | 随机字符串生成器（OAuth2 state、CSRF 等） |
| `auth-platform` | 认证平台 |
| `github-oauth2` | GitHub OAuth2 登录（自动启用 `nonce`） |
| `argon2-hash` | Argon2i/d/id 密码哈希 |
| `rate-limit` | 限流（令牌桶/滑动窗口，防刷防攻击） |

### 网络与安全

| Feature | 说明 |
|---------|------|
| `afast-tls` | HTTPS / WSS 支持（rustls + ALPN HTTP/2），独立 `[tls]` 配置段 |
| `acme` | Let's Encrypt 自动证书申请与续期（HTTP-01 验证） |

### 微信生态

| Feature | 说明 |
|---------|------|
| `wx-login-mini` | 微信小程序登录 |
| `wx-login-app` | 微信 APP 登录 |
| `wx-login-web` | 微信网页扫码登录 |
| `wx-official` | 微信公众号管理（模板消息、菜单、素材、网页授权登录） |
| `wx-virtual-pay` | 微信虚拟支付 |
| `wx-sec-check` | 微信内容安全检测 |
| `wx-pay-h5` | 微信 H5 支付 |
| `wx-pay-native` | 微信 Native 支付（PC 扫码） |
| `wx-pay-app` | 微信 APP 支付 |
| `wx-pay-mini` | 微信小程序支付 |
| `wx-pay-js` | 微信 JSAPI 支付 |

### 推送服务

| Feature | 说明 |
|---------|------|
| `push` | 推送中心基础（ID/分组/标签管理） |
| `push-getui` | 个推推送（自动启用 `push`） |
| `push-jpush` | 极光推送（自动启用 `push`） |
| `push-xiaomi` | 小米推送（自动启用 `push`） |

### 存储与文件

| Feature | 说明 |
|---------|------|
| `oss` | 阿里云 OSS 对象存储 |
| `cos` | 腾讯云 COS 对象存储 |
| `file` | 本地文件服务（上传/下载/预览） |
| `serve` | 静态网页服务（Vue/React SPA，运行时目录 / 编译期嵌入） |
| `serve-embed` | 编译期嵌入整个目录到二进制文件 |
| `image` | 图片生成（PNG/JPEG/WebP） |
| `excel` | Excel / CSV 导入导出 |
| `pdf` | PDF 生成 |

### 支付

| Feature | 说明 |
|---------|------|
| `ali-pay-web` | 支付宝电脑网站支付（RSA2 签名） |

### 权限

| Feature | 说明 |
|---------|------|
| `rbac` | RBAC 权限管理（默认角色 + 自定义角色） |

### 数据与缓存

| Feature | 说明 |
|---------|------|
| `db-postgres` | PostgreSQL 数据库 |
| `db-sqlite` | SQLite 数据库 |
| `db-mysql` | MySQL 数据库 |
| `redis` | Redis 客户端 |
| `valkey` | Valkey 客户端（与 Redis 共享实现） |
| `memkv` | 内存 KV 数据库（String/Hash/List/Set/ZSet） |

> **提示**：`db-postgres`、`db-sqlite`、`db-mysql` 可同时启用。单数据库时可用 `state.db.pool()`，多数据库时用 `state.db.pg()` / `state.db.sqlite()` / `state.db.mysql()`。

### 消息通知

| Feature | 说明 |
|---------|------|
| `email` | 邮件发送（SMTP） |
| `sms-ali` | 阿里云短信（纯 Rust，无 SDK） |
| `sms-tencent` | 腾讯云短信（纯 Rust，无 SDK） |

### 地图服务

| Feature | 说明 |
|---------|------|
| `amap` | 高德地图（地理编码、路径规划、POI、天气） |
| `tmap` | 腾讯地图（地理编码、路径规划、POI、天气、IP 定位） |

### 工具

| Feature | 说明 |
|---------|------|
| `socket-binary` | 二进制 WS 长连接 |
| `socket-ws` | 普通 WS 长连接 |
| `sse` | Server-Sent Events |
| `scheduler` | 定时任务调度器（Cron 表达式） |
| `snow` | Snowflake ID 生成器 |
| `clock` | 时钟工具 |
| `regex-util` | 正则工具（常用验证 + 通用匹配） |
| `log` | 日志（tracing） |
| `tracing` | 链路追踪 |
