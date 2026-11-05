# ORM

AFaster 的 ORM 集成基于独立发布的 [`afastorm`](https://crates.io/crates/afastorm) crate。它以 SQLx 为数据库驱动，提供模型、类型化仓库、查询构造、事务、迁移和可选管理端。数据库连接池由 ORM 持有，业务代码通过 `state.orm` 使用仓库，不必为每次操作重复传入 pool。

ORM 与 [`database` 模块](../data/database.md)用途不同：

- `orm-sqlite` / `orm-postgres`：使用模型和 Repository 完成常见 CRUD、类型化查询和迁移。
- `db-sqlite` / `db-postgres` / `db-mysql`：直接使用 SQLx pool 编写 SQL，适合需要完整控制 SQL 或使用 ORM 未覆盖数据库的场景。

目前 AFaster 的 ORM 集成内置 SQLite 和 PostgreSQL。每个 AFaster 实例配置一个 ORM 驱动；更换驱动时请使用独立的迁移历史。

## 按需启用

| Feature | 功能 |
|---|---|
| `orm-sqlite` | SQLite ORM 驱动 |
| `orm-postgres` | PostgreSQL ORM 驱动 |
| `orm-admin` | 挂载基于模型属性配置的管理端；需要同时启用一个 ORM 驱动 |
| `orm-migrate` | 注册 `orm makemigrations` 和 `orm migrate` 命令 |

示例依赖：

```toml
[dependencies]
afaster = { version = "0.0.9", default-features = false, features = [
    "afast-http",
    "orm-sqlite",
    "orm-admin",
    "orm-migrate",
] }
afastorm = { version = "0.0.1", default-features = false, features = [
    "derive",
    "sqlite",
    "migrate",
] }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

使用 PostgreSQL 时，将驱动 feature 换成 `orm-postgres`，并在 `afastorm` 中启用 `postgres`。管理端、迁移等可选能力根据需要启用。

## 文档导航

- [接入与配置](./setup.md)：Feature、TOML 配置、应用启动。
- [模型与 CRUD](./models.md)：模型定义、字段属性、Repository、软删除和版本锁。
- [查询与事务](./queries.md)：条件查询、`find_by`、`find_json` 和事务。
- [迁移](./migrations.md)：生成、审查和应用迁移。
- [自动管理端](./admin.md)：模型配置、访问地址和安全建议。

完整示例见仓库中的 [`example/`](../../../example/README.md)。
