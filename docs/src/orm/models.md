# 模型与 CRUD

模型使用 `afastorm::Model` derive 声明。通过 `ModelRegistry` 注册后，模型元数据会用于建表、Repository 查询、迁移和可选管理端。

## 声明模型

```rust,ignore
use afastorm::{FromRow, Model};

#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "users")]
pub struct User {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    #[orm(unique, find_by)]
    pub username: String,
    pub email: String,
    pub created_at: String,
}
```

`table` 指定表名，未指定时由模型默认名称推导。字段类型必须是 `afastorm` 支持的类型；`Option<T>` 表示 nullable 字段。`primary_key` 声明主键，`auto_increment` 表示数据库生成主键。`unique` 用于 schema 元数据和建表定义。

可通过 `ModelRegistry::new().register::<User>()` 注册单个模型，或在统一函数中注册多个模型：

```rust,ignore
use afastorm::ModelRegistry;

pub fn model_registry() -> ModelRegistry {
    ModelRegistry::new()
        .register::<User>()
        .register::<Post>()
}
```

实际方法签名以当前使用的 `afastorm` 版本为准。

## Repository CRUD

```rust,ignore
use crate::models::User;

let repository = state.orm.repo::<User>()?;

// 插入：自动递增主键写回 model.id
let mut user = User {
    id: None,
    username: "ada".to_owned(),
    email: "ada@example.com".to_owned(),
    created_at: String::new(),
};
repository.insert(&mut user).await?;

// 主键查找；不存在时返回 None
let found = repository.find_by_id(user.id.expect("generated id")).await?;

// 按条件查找第一条
let found = repository
    .first_where(User::username.eq("ada".to_owned()))
    .await?;

// 保存：无主键时插入，有主键时更新
repository.save(&mut user).await?;

// 删除并返回受影响行数
let deleted = repository.delete(user.id.expect("generated id")).await?;
```

`insert`、`save`、`update`、`delete` 等方法返回受影响行数（`u64`）。模型必须注册到当前 ORM registry 后才能创建对应 Repository。

## 字段与生命周期属性

| 属性 | 用途 |
|---|---|
| `#[orm(column = "db_name")]` | 使用不同于 Rust 字段名的数据库列名 |
| `#[orm(primary_key)]` | 主键 |
| `#[orm(auto_increment)]` | 自增主键，须与 `primary_key` 一起使用 |
| `#[orm(unique)]` | 唯一约束 |
| `#[orm(auto_create)]` | 插入时使用数据库当前时间 |
| `#[orm(auto_update)]` | 更新时使用数据库当前时间 |
| `#[orm(soft_delete)]` | 软删除时间字段，字段必须为 `Option<T>` |
| `#[orm(version)]` | 乐观锁版本号，必须是非 nullable `i32` 或 `i64` |

自动时间字段需要使用数据库方言支持的时间类型，例如 SQLite 下的 `String` 时间列。版本字段在插入时初始化；更新、软删除和恢复会递增版本。

### 软删除

```rust,ignore
#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "posts")]
pub struct Post {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    pub title: String,
    #[orm(soft_delete)]
    pub deleted_at: Option<i64>,
    #[orm(version)]
    pub version: i64,
}
```

配置 `soft_delete` 后，Repository 查询、`count` 和默认删除会忽略/标记软删除记录，不会立即物理删除。可使用 `restore(id)` 恢复、`force_delete(id)` 物理删除。启用版本锁时也可使用 `delete_versioned(id, expected_version)` 和 `restore_versioned(id, expected_version)`；版本不匹配会返回乐观锁错误。

使用 `delete_where` 时需格外谨慎：它执行条件删除 SQL，不应用于未经筛选的生产数据操作。

## 建表

开发或测试中可调用 `orm.create_registered_tables().await?` 根据 registry 创建表。正式部署建议使用版本化迁移，见[迁移文档](./migrations.md)；该方法不会替代迁移历史管理。
