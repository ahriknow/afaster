# AFaster

基于 Rust (afast) 的高性能后端框架，内置 25+ 业务模块，覆盖认证、支付、推送、存储、地图等常见后端需求。

## 特性

- **开箱即用** — 配置文件驱动，feature 开关控制，按需编译
- **微信生态全覆盖** — 小程序/APP/网页扫码/公众号登录、微信支付 V3、公众号管理
- **多平台推送** — 个推/极光/小米，统一 API，厂商通道自动配置
- **存储与文件** — 阿里云 OSS/腾讯云 COS/本地文件/PDF/Excel/图片生成
- **安全认证** — JWT/Argon2/OAuth2/限流/短信验证码
- **数据层** — PostgreSQL/SQLite/MySQL/Redis/Valkey

## 快速开始

```toml
# Cargo.toml
[dependencies]
afaster = { version = "0.0.4", features = ["jwt", "log"] }
```

```toml
# config.toml
[backend]
host = "0.0.0.0"
port = 5000
```

详见 [快速开始](./quick-start.md)
