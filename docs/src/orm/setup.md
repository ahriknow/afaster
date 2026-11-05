# 接入与配置

> Features：`orm-sqlite` 或 `orm-postgres`；可选 `orm-admin`、`orm-migrate`

## 依赖 Feature

AFaster 的 ORM 驱动通过 Feature 选择。应用只应在配置中使用已启用的驱动，并且每个 AFaster 实例只能配置一个 ORM 数据库。

```toml
[dependencies]
afaster = { version = "0.0.9", default-features = false, features = [
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

若要同时启用 PostgreSQL 和 SQLite 的代码路径，可在两边都启用对应 feature，但单个运行实例仍只能配置一个 ORM section。`orm-admin` 需要 `orm-sqlite` 或 `orm-postgres`；`orm-migrate` 会启用命令行支持。

## SQLite 配置

```toml
[orm-sqlite]
path = "data/app.db"

# 启用 orm-admin 时配置：
token = "replace-with-a-long-random-secret"
url = "/admin"
```

`path` 是必填项。`:memory:` 用于内存数据库；以 `sqlite:` 开头的值会作为 SQLite URL 使用。`token` 和 `url` 可选，仅用于 ORM 管理端。

## PostgreSQL 配置

```toml
[orm-postgres]
host = "127.0.0.1"
port = 5432
user = "app"
pass = "replace-me"
name = "app_db"

# 启用 orm-admin 时配置：
token = "replace-with-a-long-random-secret"
url = "/admin"
```

`host`、`port`、`user`、`pass`、`name` 为连接配置；密码和管理端 token 应通过部署环境提供，不要提交真实凭据。

## 初始化应用并注册模型

模型需要先加入 registry，再由 AFaster 状态持有 ORM。业务 handler 从 `AppState` 获取连接和 repository：

```rust,ignore
use afaster::{AFaster, AFasterOrmExt};

#[tokio::main]
async fn main() {
    let app = AFaster::new("config.toml".to_owned())
        .await
        .expect("initialize application")
        .with_orm(|orm| orm.with_models(model_registry()))
        .expect("configure ORM");

    app.run().await;
}
```

管理端启用时，在配置 section 中设置 `token`，并显式启用管理端：

```rust,ignore
.with_orm(|orm| {
    orm.with_models(model_registry())
        .with_admin(true)
})
```

也可以通过 `.with_admin_url("/backoffice")` 和 `.with_admin_token("...")` 在代码中覆盖 URL 与 token。

在 handler 中，`state.orm.repo::<User>()` 获取模型 Repository；ORM 内部管理连接池，无需额外传入 pool：

```rust,ignore
let user = state
    .orm
    .repo::<User>()?
    .find_by_id(user_id)
    .await?;
```

如果业务需要直接写 SQL，可以使用 `state.orm.inner()` 获取底层 `afastorm::Orm`。不要将用户输入拼接进 SQL；使用 ORM 的绑定参数和条件构造器。
