# Redis / Valkey

Feature: `redis` / `valkey` | 依赖: `redis`

> 📖 官方文档：Redis <https://redis.io/docs/latest/> | Valkey <https://valkey.io/docs/>

## 简介

Redis 和 Valkey 客户端封装，两者共享同一实现（协议完全兼容）。支持基础键值操作、Hash 操作、原子计数、分布式锁和发布/订阅。

## 配置

`redis` 和 `valkey` 互斥，同时只能启用一个。

```toml
# Redis
[redis]
host = "127.0.0.1"  # 主机地址, 默认 127.0.0.1
port = 6379         # 端口, 默认 6379
db = 0              # 数据库编号, 默认 0
password = ""       # 密码, 为空时不使用密码连接
prefix = ""         # Key 前缀, 用于多租户隔离

# 或者 Valkey（配置项完全相同）
[valkey]
host = "127.0.0.1"
port = 6379
db = 0
password = ""
prefix = ""
```

## API

### 基础操作

```rust
// GET
let val: Option<String> = redis.get("key").await?;
let num: Option<i64> = redis.get("counter").await?;

// SET（无过期）
redis.set("key", "value", None).await?;

// SET（带过期，秒）
redis.set("key", "value", Some(3600)).await?;

// SET NX — 仅当 key 不存在时设置
let ok = redis.set_nx("key", "value", Some(300)).await?;

// DEL
let deleted = redis.del("key").await?;

// EXISTS
let exists = redis.exists("key").await?;

// EXPIRE
redis.expire("key", 3600).await?;

// TTL
let ttl = redis.ttl("key").await?;  // -1 永不过期, -2 不存在

// KEYS（慎用，生产环境建议用 SCAN）
let keys = redis.keys("user:*").await?;

// INCR — 原子自增
let count = redis.incr("counter", 1).await?;
let count = redis.incr("counter", -1).await?;  // 自减
```

### Hash 操作

```rust
// HSET
redis.hset("user:1", "name", "Alice").await?;

// HGET
let name: Option<String> = redis.hget("user:1", "name").await?;

// HDEL
redis.hdel("user:1", "name").await?;

// HGETALL
let all: HashMap<String, String> = redis.hgetall("user:1").await?;

// HINCRBY
let score = redis.hincrby("user:1", "score", 10).await?;
```

### 分布式锁

```rust
use uuid::Uuid;

let lock_value = Uuid::new_v4().to_string();

// 获取锁（自动过期 30 秒）
let acquired = redis.lock("order:123", &lock_value, 30).await?;

if acquired {
    // 执行临界区操作
    // ...

    // 释放锁（仅当 value 匹配时释放，防止误释放）
    redis.unlock("order:123", &lock_value).await?;
}
```

### 发布/订阅

```rust
// 发布消息
let subscribers = redis.publish("channel:news", "hello").await?;
```

## Key 前缀

配置 `prefix` 后，所有 key 自动添加前缀：

```toml
[redis]
host = "127.0.0.1"
port = 6379
prefix = "myapp"
```

```rust
redis.set("user:1", "Alice", None).await?;
// 实际存储的 key 是 "myapp:user:1"
```

适用于多租户或多应用共享同一 Redis 实例的场景。

## 错误码

| 错误码 | 说明 |
|--------|------|
| 51601 | 连接失败（地址错误、服务不可达） |
| 51602 | 连接断开 |
| 51603 | 命令执行失败 |
| 51602 | 连接断开（运行中丢失连接） |
| 51603 | 命令执行失败 |

## 注意事项

- `redis` 和 `valkey` feature 互斥，底层使用同一个 `redis` crate
- `keys()` 命令在大量 key 时会阻塞，生产环境建议用 `SCAN`
- 分布式锁使用 Lua 脚本保证原子性
- 连接使用 `MultiplexedConnection`，支持并发请求
