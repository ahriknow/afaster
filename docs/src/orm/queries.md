# 查询与事务

## 条件查询

Repository 的 `query()` 返回类型化查询构造器，可选择字段、过滤、排序、分页后生成 `Statement`，再由 Repository 将结果解码为模型：

```rust,ignore
use afastorm::{Field, SortDirection};

let mut repository = state.orm.repo::<Post>()?;
let statement = repository
    .query()
    .filter(Field::new("status").eq(2))
    .order_by_name("created_at", SortDirection::Desc)
    .limit(20)
    .offset(0)
    .build()?;
let posts: Vec<Post> = repository.fetch_all(&statement).await?;
```

Repository 还提供 `all()`、`find_by_id(id)`、`first_where(condition)`、`count()` 和 `count_where(condition)` 等快捷方法。启用软删除的模型，其默认 Repository 查询会自动排除已删除记录。

条件中的列名和排序字段应来自模型定义或受控的服务端代码，不要直接用未经校验的客户端输入构造字段名或 SQL。分页时设置合理的 `limit`，并选择稳定的排序字段。

## 生成式 `find_by`

在模型字段上标注 `#[orm(find_by)]`，或在模型属性中声明字段组合：

```rust,ignore
#[derive(Clone, Debug, FromRow, Model)]
#[orm(table = "posts", find_by(author_id, slug))]
pub struct Post {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    pub author_id: i64,
    #[orm(find_by)]
    pub slug: String,
    pub title: String,
}
```

derive 会生成 `PostFindBy` trait。导入 trait 后即可调用：

```rust,ignore
use crate::models::{Post, PostFindBy as _};

let repository = state.orm.repo::<Post>()?;
let by_slug = repository.find_by_slug("hello-world".to_owned()).await?;
let by_owner = repository
    .find_by_author_id_slug(author_id, "hello-world".to_owned())
    .await?;
```

方法返回 `Result<Option<Post>>`。字段组合的顺序决定方法参数顺序；同名生成方法不能重复声明。属性只接受模型中存在的字段，错误会在编译期报告。

## JSON / JSONB 查找

仅 `serde_json::Value` 和 `Option<serde_json::Value>` 字段可声明 `find_json`。路径以模型字段为起点，支持嵌套 JSON 键：

```rust,ignore
#[derive(Clone, Debug, FromRow, Model)]
#[orm(
    table = "posts",
    find_json(metadata.featured, eq),
    find_json(metadata, contains, has_key)
)]
pub struct Post {
    #[orm(primary_key, auto_increment)]
    pub id: Option<i64>,
    pub metadata: serde_json::Value,
}
```

示例调用：

```rust,ignore
use crate::models::{Post, PostFindJson as _};

let repository = state.orm.repo::<Post>()?;
let featured = repository
    .find_json_eq_metadata_featured(serde_json::json!(true))
    .await?;
let containing_topic = repository
    .find_json_contains_metadata(serde_json::json!({"topics": ["rust"]}))
    .await?;
let with_key = repository
    .find_json_has_key_metadata("topics".to_owned())
    .await?;
```

生成方法返回 `Result<Option<Post>>`。`find_json` 的操作列表决定实际生成的方法；嵌套字段会按路径生成方法名（例如 `find_json_eq_metadata_featured`）。支持的操作及数据库行为详见 [`afastorm` CRUD 文档](https://github.com/ahriknow/afastorm/tree/main/docs)。

SQLite JSON 查找依赖 SQLite JSON1 函数。PostgreSQL JSON 查询通过 JSONB 操作表达式实现；相关数据库列应使用兼容的 JSON / JSONB 类型。

## 原生参数化 SQL

需要复杂查询时，可使用底层 `afastorm::Orm` 的 `query_as`、`query`：

```rust,ignore
let row: (i64,) = state
    .orm
    .inner()
    .query_as("SELECT COUNT(*) FROM posts WHERE status = ?")
    .bind(2_i64)
    .fetch_one()
    .await?;
```

SQL 文本须为开发者编写的静态字符串，值通过 `bind` 传递。不要将用户输入拼接进 SQL。

## 事务

用事务将多次写入组合为一个原子操作；闭包返回 `Err` 时 ORM 会回滚，正常返回时提交：

```rust,ignore
let post_id = state.orm.inner().transaction(async |transaction| {
    let mut post = Post { /* fields */ };
    transaction.repo::<Post>()?.insert(&mut post).await?;
    let post_id = post.id.ok_or_else(|| {
        afastorm::Error::Query("post ID was not generated".to_owned())
    })?;

    transaction.repo::<PostTag>()?
        .insert(&mut PostTag { post_id, tag_id })
        .await?;
    Ok(post_id)
}).await?;
```

也可以使用 `begin()` 显式控制事务，并调用 `commit()` 或 `rollback()`。事务 Repository 提供与普通 Repository 对应的模型操作。

事务适用于必须一起成功或失败的数据库操作。避免在事务中执行不必要的网络请求或长时间等待，以免长时间占用数据库连接。
