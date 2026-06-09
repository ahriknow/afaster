# 数据库

Feature: `db-postgres` / `db-sqlite` / `db-mysql`（三者互斥，只能启用一个）

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
| `connect_postgres` | `config: &PostgresConfig` | `Result<Database>` | 建立 PostgreSQL 连接池 |
| `connect_sqlite` | `config: &SqliteConfig` | `Result<Database>` | 建立 SQLite 连接池 |
| `connect_mysql` | `config: &MysqlConfig` | `Result<Database>` | 建立 MySQL 连接池 |

### Database 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `pool` | `PgPool` / `SqlitePool` / `MySqlPool` | sqlx 连接池，按 feature 决定类型 |

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 50901 | Database connection failed | 数据库连接失败 |
| 50902 | Database config error | 数据库配置错误 |

## 使用示例

```rust
// 数据库已通过 AppState 自动初始化
// 在 handler 中通过 state.db.pool 使用 sqlx 查询

#[afast::post("/users")]
async fn get_user(state: &AppState, body: UserId) -> Result<User> {
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", body.id)
        .fetch_one(&state.db.pool)
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
