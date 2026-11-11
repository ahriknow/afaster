# AFaster

基于 [afast](https://crates.io/crates/afast) 的 Rust 后端框架。AFaster 将网络服务、认证、安全、数据库、云服务和文件处理等常见能力拆分为可选模块，通过 Cargo features 按需组合。

- **按需启用**：默认不启用业务模块，避免引入不需要的依赖。
- **异步运行**：基于 Tokio 和 afast。
- **多种服务能力**：支持 HTTP、WebSocket、TCP、TLS 等 afast 提供的传输与代码生成能力。
- **常用集成**：覆盖微信、阿里云、腾讯云、GitHub、推送、数据库、缓存和文件处理。
- **ORM 可选集成**：使用独立发布的 [`afastorm`](https://crates.io/crates/afastorm)，支持 SQLite、PostgreSQL、迁移和模型管理端。

## 快速开始

```toml
# Cargo.toml
[dependencies]
afaster = { version = "0.0.10", default-features = false, features = ["afast-http"] }
tokio = { version = "1", features = ["full"] }
```

```rust
use afaster::AFaster;

#[tokio::main]
async fn main() {
    let app = AFaster::new("config.toml".to_owned())
        .await
        .expect("initialize AFaster");
    app.run().await;
}
```

更多实际配置见 [`config.toml`](./config.toml)，完整模块说明见 [在线文档](https://ahriknow.github.io/afaster/)。

## 可选模块

所有模块均通过 Cargo features 控制。请只启用所需 feature；一些子 feature 会自动启用其依赖的基础 feature。

### 服务与安全

| Feature | 功能 |
|---|---|
| `afast-http`、`afast-tcp`、`afast-ws` | afast HTTP、TCP、WebSocket 传输 |
| `afast-tls` | 基于 rustls 的 TLS |
| `afast-ts`、`afast-js`、`afast-kt`、`afast-rs` | 生成 TypeScript、JavaScript、Kotlin、Rust 客户端 |
| `afast-code`、`afast-doc` | 代码生成和 API 文档 |
| `jwt`、`nonce`、`argon2-hash` | JWT、随机 nonce、Argon2 密码哈希 |
| `rbac`、`rate-limit` | 角色权限控制和限流 |
| `acme` | Let's Encrypt HTTP-01 证书申请与续期 |
| `github-oauth2` | GitHub OAuth2 登录 |

### 数据、缓存和迁移

| Feature | 功能 |
|---|---|
| `db-postgres`、`db-sqlite`、`db-mysql` | 直接访问 SQLx 连接池 |
| `orm-sqlite`、`orm-postgres` | afastorm 的 SQLite、PostgreSQL 驱动 |
| `orm-chrono`、`orm-time` | afastorm 的 chrono / time 字段类型支持 |
| `orm-admin` | AFaster 集成的模型管理页面和 API |
| `orm-migrate` | 注册 `orm makemigrations` / `orm migrate` 命令 |
| `redis`、`valkey` | Redis / Valkey |
| `memkv`、`bloom` | 内存 KV 和 Bloom Filter |

`database` 适合直接编写 SQL 并使用连接池；ORM 适合使用模型、Repository 和迁移的场景。两者可以按应用需求选择。AFaster ORM features 使用 crates.io 上独立发布的 `afastorm` crate；单个运行实例配置一个 ORM 驱动。

### 云服务、文件和工具

| 类别 | Features |
|---|---|
| 微信 | `wx-login-mini`、`wx-login-app`、`wx-login-web`、`wx-official`、`wx-pay-h5`、`wx-pay-native`、`wx-pay-app`、`wx-pay-mini`、`wx-pay-js`、`wx-virtual-pay`、`wx-sec-check` |
| 阿里云 | `ali-pay-web`、`oss`、`sms-ali` |
| 腾讯云 | `cos`、`sms-tencent`、`tmap` |
| 推送与通知 | `push`（个推、极光、小米）、`email`、`sse` |
| 文件与内容 | `file`、`serve`、`serve-embed`、`image`、`excel`、`pdf` |
| 其他 | `amap`、`trace`、`scheduler`、`snow`、`clock`、`regex-util`、`log` |

功能列表以 [`Cargo.toml`](./Cargo.toml) 的 `[features]` 为准；各模块的配置和 API 见 [模块文档](./docs/src/SUMMARY.md)。

## ORM 使用

增加 ORM 依赖和 feature：

```toml
afaster = { version = "0.0.10", default-features = false, features = [
    "afast-http",
    "orm-sqlite",
    "orm-migrate",
] }
afastorm = { version = "0.0.1", default-features = false, features = [
    "derive",
    "sqlite",
    "migrate",
] }
```

构造应用时注册模型，并在启动副作用前处理 ORM CLI 命令：

```rust,ignore
use afaster::{AFaster, AFasterOrmExt};

let app = AFaster::new("config.toml".to_owned())
    .await?
    .with_orm(|orm| orm.with_models(model_registry()))?;

if app.run_command_if_requested().await? {
    return Ok(());
}

app.run().await;
```

在 handler 中使用 `state.orm.repo::<Model>()` 操作模型，无需每次传入 pool。启用 `orm-admin` 后，还可以通过模型属性配置管理端；管理端必须设置强随机 token，并限制网络访问。

```sh
cargo run -- orm makemigrations add_profile --dir migrations
cargo run -- orm migrate --dir migrations
```

详见 [ORM 文档](./docs/src/orm/README.md)及[博客示例](./example/README.md)。迁移生成结果应在应用前审查；应用启动不会自动执行迁移。

## 静态文件服务

启用 `serve` 可从运行时目录提供静态文件和 SPA 页面：

```rust,ignore
use afaster::{AFaster, AFasterServeExt, serve::Serve};

let app = AFaster::new("config.toml".to_owned())
    .await?
    .with_serve(Serve::from_dir("./dist").with_spa(true));
```

还可以启用 `serve-embed`，使用 `Serve::from_embedded(...)` 在编译期嵌入静态资源。

## 配置与扩展

功能模块通常通过 `config.toml` 初始化；支持构建器扩展的模块使用对应 extension trait，例如：

```rust,ignore
use afaster::{AFaster, AFasterRbacExt};

let app = AFaster::new("config.toml".to_owned())
    .await?
    .with_rbac(|rbac| rbac.with_store(MyRbacStore::new()))?;
```

应用也可通过 `CommandModule` 注册自定义命令。配置格式和模块扩展方式请查阅对应文档。

## 文档与示例

- [文档主页与目录](./docs/src/SUMMARY.md)
- [ORM 接入](./docs/src/orm/README.md)
- [博客系统示例](./example/README.md)
- [配置示例](./config.toml)

本地预览 mdBook 文档：

```sh
mdbook serve docs -p 5500
```

## 环境与许可

- Rust 2024 edition
- Tokio async runtime
- MIT License
