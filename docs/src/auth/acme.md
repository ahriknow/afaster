# ACME 自动证书管理

自动申请和续期 Let's Encrypt TLS 证书，使用 HTTP-01 验证方式。

## Feature

启用 `acme` feature 后自动生效。需要同时启用 `afast-tls` 以提供 HTTPS 服务。

## 配置

```toml
[acme]
domains = ["a.domain.com", "b.domain.com"]   # 申请证书的域名列表（多域名生成一个 SAN 证书）
contact = "mailto:email@domain.com"           # 联系邮箱（可选，用于证书到期提醒）
cache_dir = "./acme_cache"                    # 证书缓存目录，默认 ./acme_cache
# staging = false                             # 是否使用 Let's Encrypt 测试环境，默认 false
# renewal_days = 10                           # 到期前多少天自动续期，默认 10
# allow_non_80 = false                        # 允许非 80 端口，默认 false
                                              # 设为 true 时需自行配置反向代理将 80 转发到服务端口
```

## TLS 配置

ACME 模块申请的证书自动缓存到 `cache_dir`，TLS 模块从该目录读取：

```toml
[tls]
port = 6443                                   # HTTPS 监听端口，默认 443
cert_path = "./acme_cache/fullchain.pem"      # 证书链文件路径
key_path = "./acme_cache/privkey.pem"         # 私钥文件路径
```

## 代码示例

### 基本用法

```rust,no_run
use afaster::{AFaster, AFasterAcmeExt, AFasterServeExt};

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".into()).await
        .unwrap()
        .with_acme(|acme| acme)
        .run()
        .await;
}
```

### 注册回调（证书热重载）

```rust,no_run
use afaster::{AFaster, AFasterAcmeExt, AFasterServeExt};

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".into()).await
        .unwrap()
        .with_acme(|acme| {
            acme.with_on_cert_obtained(|(state, _event)| async move {
                state.tls.reload();  // 首次申请成功后触发 HTTPS 热重载
                Ok(())
            })
            .with_on_cert_renewed(|(state, _event)| async move {
                state.tls.reload();  // 续期成功后触发 HTTPS 热重载
                Ok(())
            })
            .with_on_cert_failed(|(_state, event)| async move {
                eprintln!("证书申请/续期失败: {}", event.error);
                Ok(())
            })
        })
        .run()
        .await;
}
```

## 工作流程

### 启动时

1. 检查缓存目录是否存在有效证书
2. **缓存命中** → 立即检查证书是否即将到期
   - 到期 → 删除旧证书，触发重新申请
   - 未到期 → 启动续期后台任务
3. **缓存未命中** → 在后台异步申请证书（等待 HTTP 服务就绪后开始）

### 续期后台任务

- 立即检查一次（处理重启后证书即将到期的情况）
- 之后每 24 小时检查一次证书到期时间
- 到期前 `renewal_days` 天自动续期
- 续期成功后调用 `on_cert_renewed` 回调

### HTTP-01 验证

ACME 模块自身不启动网络监听。验证请求通过 afast HTTP 服务的路由处理：

- 路由：`/.well-known/acme-challenge/*`
- Let's Encrypt 请求该路径时，从 `ChallengeStore` 中查找对应的 `key_authorization` 并返回

## 注意事项

- HTTP-01 验证要求 Let's Encrypt 能访问 80 端口
- 如果 `backend.port` 不是 80，需要设置 `allow_non_80 = true` 并自行配置反向代理
- 首次申请需要 HTTP 服务已启动（模块内部等待 2 秒后开始申请）
- 证书文件缓存在 `cache_dir` 目录下：`fullchain.pem`（证书链）和 `privkey.pem`（私钥）
