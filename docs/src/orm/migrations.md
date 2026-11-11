# 迁移

> Feature：`orm-migrate`

迁移由开发者审查的 SQL 文件管理。应用启动时不会自动执行迁移。命令需要在创建 AFaster 应用并注册模型后调用 `run_command_if_requested()`，以便迁移生成器读取当前模型 registry。

## 注册 ORM 命令

```rust,ignore
use afaster::{AFaster, AFasterOrmExt};

#[tokio::main]
async fn main() -> afaster::Result<()> {
    let app = AFaster::new("config.toml".to_owned())
        .await?
        .with_orm(|orm| orm.with_models(model_registry()))?;

    if app.run_command_if_requested().await? {
        return Ok(());
    }

    app.run().await;
    Ok(())
}
```

`orm-migrate` 会将 `orm` 命令组加入应用 CLI。应在启动服务、播种数据或执行其他有副作用的初始化前处理命令。
`AFaster::new` 仍会先按配置初始化 ORM 连接；命令在 `run_command_if_requested()` 中执行，处理后提前退出，不会继续启动服务。运行 `makemigrations` 前必须通过 `with_orm` 注册模型。

## 生成迁移

```sh
cargo run -- orm makemigrations add_post_status --dir migrations
```

或在 workspace 示例中：

```sh
cargo run -p example -- orm makemigrations add_post_status --dir migrations
```

首次生成时，工具会创建 schema 快照和迁移文件。后续会将模型 registry 与 `schema.json` 对比，生成 `.up.sql` 和 `.down.sql`。若 schema 没有变化，会提示 `no schema changes found`。

`--dir` 默认为 `migrations`，相对路径以当前工作目录解析。迁移名称会成为文件名的一部分，建议使用简短、可读的 snake_case 名称。

生成后务必审查迁移 SQL 与快照，然后将它们和模型变更一并提交。生成器不会自动应用新迁移。

## 应用迁移

```sh
cargo run -p example -- orm migrate --dir migrations
```

迁移文件名使用时间版本和名称，`.up.sql` 文件按版本顺序执行。成功应用的版本会被记录，重复运行时不会再次执行已应用迁移。SQLite 和 PostgreSQL 的迁移语句在事务中执行；失败的迁移会回滚。
`migrate` 不要求注册模型，但应用仍需正确配置 ORM 驱动和数据库连接。应用启动本身不会自动创建表或运行迁移。

## 生成器边界

- 创建新表、增加 nullable 普通列，以及移除 nullable 且非 unique、无额外约束的列可由生成器处理。
- 新增非空列必须先设计默认值或数据回填，生成器会要求手写迁移。
- 修改约束、unique、主键或不支持安全判断的列变化，需要手写并审查迁移。
- 删除列的 down migration 会重新创建列，但无法恢复已删除的数据。
- PostgreSQL 部分类型变更会生成显式 cast；应先验证旧数据可转换，再应用迁移。
- SQLite 与 PostgreSQL 使用不同 SQL 方言；不要在不同驱动间共用同一迁移目录或 schema 历史。

## 建议流程

1. 修改模型并注册。
2. 运行 `makemigrations` 生成 SQL。
3. 审查 up/down 文件，必要时改为安全的手写迁移。
4. 在开发或测试数据库运行 `migrate`，验证升级和回滚策略。
5. 将模型、迁移文件和 `schema.json` 一起提交。
6. 部署时显式执行 `migrate`，然后启动应用。
