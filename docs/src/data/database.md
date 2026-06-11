# 数据库

Feature: `db-postgres` / `db-sqlite` / `db-mysql`（可同时启用多个）

## 配置

```toml
# PostgreSQL
[postgres]
host = "127.0.0.1"
port = 5432
user = "postgres"
pass = ""
name = "afaster"

# SQLite
[sqlite]
path = "data.db"   # 留空则使用内存数据库

# MySQL
[mysql]
host = "127.0.0.1"
port = 3306
user = "root"
pass = ""
name = "afaster"
```

## API

### Database

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `pg()` | - | `&PgPool` | 获取 PostgreSQL 连接池 |
| `sqlite()` | - | `&SqlitePool` | 获取 SQLite 连接池 |
| `mysql()` | - | `&MySqlPool` | 获取 MySQL 连接池 |
| `pool()` | - | `&PgPool` / `&SqlitePool` / `&MySqlPool` | 获取默认连接池（仅启用单个数据库时可用） |

### Database 字段

| 字段 | 类型 | Feature | 说明 |
|------|------|---------|------|
| `pg` | `PgPool` | `db-postgres` | PostgreSQL 连接池 |
| `sqlite` | `SqlitePool` | `db-sqlite` | SQLite 连接池 |
| `mysql` | `MySqlPool` | `db-mysql` | MySQL 连接池 |

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 50901 | Database connection failed | 数据库连接失败 |
| 50902 | Database config error | 数据库配置错误 |

## 使用示例

```rust
// 数据库已通过 AppState 自动初始化
// 单数据库时可通过 state.db.pool() 获取连接池
// 多数据库时使用 state.db.pg() / state.db.sqlite() / state.db.mysql()

#[afast::post("/users")]
async fn get_user(state: &AppState, body: UserId) -> Result<User> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", body.id)
        .fetch_one(state.db.pool())
        .await?;
    Ok(user)
}
```

## 模块结构

```
src/state/database/
├── mod.rs   # Database 结构体、连接方法、配置结构体
└── err.rs   # 数据库错误码函数
```
