# 接入与配置

> Features：`orm-sqlite` 或 `orm-postgres`；可选 `orm-chrono`、`orm-time`、`orm-admin`、`orm-migrate`

## 依赖 Feature

AFaster 的 ORM 驱动通过 Feature 选择。应用只应在配置中使用已启用的驱动，并且每个 AFaster 实例只能配置一个 ORM 数据库。

```toml
[dependencies]
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

若要同时启用 PostgreSQL 和 SQLite 的代码路径，可在两边都启用对应 feature，但单个运行实例仍只能配置一个 ORM section。`orm-admin` 需要 `orm-sqlite` 或 `orm-postgres`；`orm-migrate` 会启用命令行支持。
`orm-chrono` 和 `orm-time` 分别为 `afastorm` 启用 `chrono` 和 `time` 类型支持。使用这些类型时，应用还应在 `afastorm` 依赖中启用对应的 `chrono` 或 `time` feature；普通类型模型不需要开启。

## SQLite 配置

```toml
[orm-sqlite]
path = "data/app.db"

# 可选的 ORM 管理端静态 token；省略或留空时不启用认证：
# token = "从部署环境注入的静态 Bearer token"
url = "/admin"
```

`path` 是必填项。`:memory:` 用于内存数据库；以 `sqlite:` 开头的值会作为 SQLite URL 使用。`token` 和 `url` 可选，仅用于 ORM 管理端。
普通文件路径会作为可读写 SQLite 数据库打开；如果配置多个已启用的 ORM 驱动 section，初始化会报错。

## PostgreSQL 配置

```toml
[orm-postgres]
host = "127.0.0.1"
port = 5432
user = "app"
pass = "replace-me"
name = "app_db"

# 可选的 ORM 管理端静态 token；省略或留空时不启用认证：
# token = "从部署环境注入的静态 Bearer token"
url = "/admin"
```

`host`、`port`、`user`、`pass`、`name` 为连接配置；密码和管理端 token 应通过部署环境提供，不要提交真实凭据。
用户名、密码和数据库名中的 URL 特殊字符会进行 percent-encoding；host 应填写主机名或 IP，不包含 scheme 和端口。

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

管理端启用时，必须显式启用管理端。静态 token 可在配置 section 中提供，也可通过 `.with_admin_token(...)` 覆盖：

```rust,ignore
.with_orm(|orm| {
    orm.with_models(model_registry())
        .with_admin(true)
})
```

管理端还必须至少向 `AdminModelRegistry` 注册一个启用了管理属性的模型，并将相同模型加入 ORM `ModelRegistry`；未注册或元数据不匹配会在配置应用时返回错误。`AFasterAdmin` derive 由 `orm-admin` feature 启用并从 `afaster::orm` 重导出，应用不需要单独添加 `afaster-derive` 依赖。

也可以通过 `.with_admin_url("/backoffice")` 和 `.with_admin_token("...")` 在代码中覆盖 URL 与 token。URL 必须是 path，不能包含 query 或 fragment。静态 token 缺省或为空时，管理端 API 不要求认证；也可以通过 `.with_admin_token_functions(generate, verify)` 使用应用自定义凭据和 token 验证。**不要将无认证的管理端暴露到不受信任的网络。** 品牌、有序 i18n、权限属性和 API 行为详见[自动管理端](./admin.md)。

在 handler 中，`state.orm.repo::<User>()` 获取模型 Repository；ORM 内部管理连接池，无需额外传入 pool：

```rust,ignore
let user = state
    .orm
    .repo::<User>()?
    .find_by_id(user_id)
    .await?;
```

如果业务需要直接写 SQL，可以使用 `state.orm.inner()` 获取底层 `afastorm::Orm`。不要将用户输入拼接进 SQL；使用 ORM 的绑定参数和条件构造器。
