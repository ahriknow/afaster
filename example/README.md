# AFaster 博客示例

这个 workspace 示例是一个基于 AFaster 的多人博客后端，用来展示 HTTP / Binary API、WebSocket、RBAC、JWT、Argon2、SQLite 和 `afastorm` ORM 的组合方式。

示例包含注册与登录、文章和分类/标签、评论、点赞、管理操作、RBAC 角色权限、WebSocket 聊天、Binary 聊天，以及公开 HTTP API。HTTP API 提供健康检查、文章/分类/标签查询、精选文章和统计等接口。

## 环境要求

- Rust toolchain（edition 2024）
- 从仓库根目录执行 Cargo 命令
- SQLite 本地文件数据库，无需另行启动数据库服务

## 首次运行

示例配置位于 [`config.toml`](./config.toml)，默认 SQLite 文件 `blog.db` 相对于仓库根目录创建，HTTP 服务默认监听 `0.0.0.0:6080`。

先应用数据库迁移：

```sh
cargo run -p example -- orm migrate --dir migrations
```

再启动服务：

```sh
cargo run -p example
```

启动后终端会打印 API 与管理端地址。示例使用默认演示管理员：

| 用户名 | 密码 |
|---|---|
| `admin` | `admin123` |

默认管理员仅用于本地演示。公开部署前必须更改管理员密码、JWT secret 和 ORM 管理端 token，并检查 CORS、监听地址和访问控制配置。

文档 API 的 Basic Auth 是示例配置值，见 `example/src/app.rs`；不要将其当作生产凭据。

## ORM 管理端

默认管理端地址为 `http://127.0.0.1:6080/admin`。管理端 Bearer token 配置在 `example/config.toml` 的 `[orm-sqlite].token`；首次访问时在页面提示框输入。

管理端首页列出已启用的模型；进入模型后，页面 URL 带有模型 hash，可直接分享或刷新恢复当前模型。支持搜索、分页及直接跳转页码；新增、编辑和查看通过带动画的弹窗完成；删除需要二次确认。操作成功或失败会显示自动关闭的提示，鼠标悬停时暂停倒计时。

示例通过 `AFasterAdmin` derive 在 AFaster 侧注册管理端元数据：

- 模型与字段分别使用 `#[admin(...)]` 配置操作能力、显示字段、顺序及 label。
- `label_lang` 从 `.with_i18ns(...)` 的有序语言翻译表中取值；第一种语言是默认语言，当前语言由 URL 的 `?lang=` 参数决定。
- ORM `ModelRegistry` 与 `AdminModelRegistry` 分别注册数据库模型和管理端元数据；`afastorm` 不再包含管理端属性。
- `#[orm(soft_delete)]` 与 `#[orm(version)]` 在管理端删除和更新中提供软删除及乐观锁。
- 文章的创建、更新时间、发布时间和软删除时间使用 chrono 日期时间类型；日期时间字段使用日期时间输入控件。

具体字段配置见 [`entities.rs`](./src/model/entities.rs)。管理端是受 Bearer token 保护的轻量工具，不替代生产系统的身份认证、权限分层和审计。不要向管理端暴露密码哈希、token 或其他敏感字段。

## 示例中的 ORM 能力

### 模型与 Repository

模型在 [`entities.rs`](./src/model/entities.rs) 中声明并加入模型注册表。应用通过 `state.orm.repo::<Model>()` 获取 Repository；连接池由 ORM 管理，调用 CRUD 无需传入 pool。

### 生成式查询

- 字段属性 `#[orm(find_by)]` 生成单字段方法，如 `find_by_username`。
- 模型属性 `find_by(author_id, slug)` 生成组合查询 `find_by_author_id_slug`。
- `find_json(metadata.featured, eq)` 和 `find_json(metadata, contains, has_key)` 生成 JSON 查找方法。

### 事务与生命周期

文章、标签及关联记录在一个事务中创建。文章模型还演示自动创建/更新时间、软删除和版本锁。ORM 集成测试覆盖事务提交与回滚、`find_by`、JSON 查询、软删除恢复和版本递增，见 [`app.rs`](./src/app.rs)。

## 数据库迁移

SQL 迁移位于仓库根目录的 `migrations/`，采用 SQLite 语法。虽然示例 crate 启用了 PostgreSQL 驱动用于 ORM 功能构建，但这组迁移不是 PostgreSQL 方言，不能直接用于 PostgreSQL。

迁移文件显式管理，应用启动不会自动建表或迁移。应用模型发生变化时，在仓库根目录运行：

```sh
cargo run -p example -- orm makemigrations describe_your_change --dir migrations
```

检查生成的 `.up.sql`、`.down.sql` 和 `migrations/schema.json` 后再应用：

```sh
cargo run -p example -- orm migrate --dir migrations
```

生成器能处理的新表、nullable 列等变化有限；新增非空列、约束变化及不明确的破坏性变更通常需要手写迁移。删除列的回滚无法恢复已删除的数据。迁移限制与建议流程见[迁移文档](../docs/src/orm/migrations.md)。

## 目录结构

```text
example/
├── config.toml
└── src/
    ├── app.rs          # 应用组装和 ORM 集成测试
    ├── bootstrap.rs    # 示例角色和管理员初始化
    ├── model/          # ORM 模型与 API 类型
    ├── rbac.rs         # SQLite RBAC 存储适配
    └── service/        # 用户、后台、管理、HTTP 和聊天服务
migrations/             # SQLite up/down SQL 和 schema 快照
```

## 运行测试

```sh
cargo test -p example
```

ORM 核心查询、derive 和 SQLite / PostgreSQL 类型测试位于独立的 `afastorm` crate 仓库。
