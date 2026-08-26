# HTTPS / WSS

Feature: `afast-tls` | 依赖: `rustls` + `tokio-rustls` + `rustls-pemfile`

## 简介

通过 `rustls` 为 HTTP 和 WebSocket 传输层提供 TLS 加密支持。

启用后，可同时监听 HTTP（明文）和 HTTPS（加密）端口。HTTPS 服务器内部自动处理 WebSocket 升级请求，无需单独配置 WSS。

支持 ALPN 协商，自动启用 HTTP/2。

## 配置

`config.toml`：

```toml
[backend]
host = "0.0.0.0"
port = 5000             # HTTP 端口

[tls]
port = 6443                       # HTTPS 端口，默认 443
cert_path = "/etc/ssl/cert.pem"   # PEM 证书链文件路径
key_path = "/etc/ssl/key.pem"     # PEM 私钥文件路径
```

> **注意**：TLS 配置已从 `[backend.tls]` 独立为顶层 `[tls]` 配置段。配合 `acme` feature 使用时，证书路径指向 ACME 缓存目录即可。

## 使用方式

```toml
[dependencies]
afaster = { version = "0.0.7", features = ["afast-http", "afast-ws", "afast-tls"] }
```

无需修改代码，框架在 `run()` 时自动检测 `[tls]` 配置，存在则启动 HTTPS 服务器：

```rust,no_run
use afaster::AFaster;

#[tokio::main]
async fn main() {
    // config.toml 中配置了 [tls] 时自动启用 HTTPS
    AFaster::new("config.toml".to_string()).await
        .unwrap()
        .run()
        .await;
}
```

## 工作原理

1. 启动时解析 `[backend.tls]` 配置
2. 使用 `rustls` 加载 PEM 证书链和私钥
3. 在 TLS 端口启动 HTTPS 服务器
4. HTTPS 服务器内部处理 HTTP 请求和 WebSocket 升级
5. HTTP 端口（`[backend].port`）仍以明文运行

## 证书

### Let's Encrypt

```bash
# 证书文件通常位于
/etc/letsencrypt/live/example.com/fullchain.pem  # cert_path
/etc/letsencrypt/live/example.com/privkey.pem    # key_path
```

### 自签名证书（开发测试）

```bash
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes
```

## Feature 依赖

| Feature | 说明 |
|---------|------|
| `afast-tls` | 启用 TLS 支持（映射到 `afast/tls`） |
| `afast-http` | HTTP 服务器（TLS 需要） |
| `afast-ws` | WebSocket 服务器（通过 HTTPS 自动支持 WSS） |
