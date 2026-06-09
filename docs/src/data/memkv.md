# MemKV 内存数据库

> Feature: `memkv`

纯内存 KV 数据库，支持与 Redis 兼容的五种数据类型：String / Hash / List / Set / ZSet。线程安全，支持 TTL 过期。暂无持久化（计划通过 ahrifs 实现）。

## 使用

```rust
use std::time::Duration;
use afaster::memkv::MemKV;

let kv = MemKV::new();

// String
kv.set("name", b"hello".to_vec(), Some(Duration::from_secs(60))).await?;
let v: Option<Vec<u8>> = kv.get("name").await?;
kv.incr("counter", 1).await?;

// Hash
kv.hset("user:1", "name", b"Alice".to_vec()).await?;
let name = kv.hget_str("user:1", "name").await?;

// List
kv.lpush("queue", b"task1".to_vec()).await?;
kv.rpush("queue", b"task2".to_vec()).await?;
let task = kv.lpop("queue").await?;

// Set
kv.sadd("tags", b"rust".to_vec()).await?;
let members = kv.smembers("tags").await?;

// ZSet
kv.zadd("leaderboard", 100.0, "player1").await?;
let top = kv.zrange("leaderboard", 0, -1).await?;
```

## API

### 通用操作

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `new()` | — | `MemKV` | 创建实例 |
| `del(key)` | `&str` | `Result<bool>` | 删除 key |
| `exists(key)` | `&str` | `Result<bool>` | key 是否存在 |
| `expire(key, ttl)` | `&str, Duration` | `Result<bool>` | 设置过期时间 |
| `ttl(key)` | `&str` | `Result<Option<Duration>>` | 获取剩余过期时间 |
| `keys(pattern)` | `&str` | `Result<Vec<String>>` | 按模式查找 key（支持 `*` 通配） |
| `typ(key)` | `&str` | `Result<Option<&str>>` | 获取 key 的类型 |
| `dbsize()` | — | `Result<usize>` | key 总数 |
| `flushdb()` | — | `Result<()>` | 清空所有数据 |

### String 操作

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get(key)` | `&str` | `Result<Option<Vec<u8>>>` | 获取值（bytes） |
| `get_str(key)` | `&str` | `Result<Option<String>>` | 获取值（UTF-8 字符串） |
| `set(key, value, ttl)` | `&str, V, Option<Duration>` | `Result<()>` | 设置值 |
| `set_nx(key, value, ttl)` | `&str, V, Option<Duration>` | `Result<bool>` | 仅当 key 不存在时设置 |
| `incr(key, delta)` | `&str, i64` | `Result<i64>` | 原子自增 |

### Hash 操作

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `hget(key, field)` | `&str, &str` | `Result<Option<Vec<u8>>>` | 获取字段值 |
| `hget_str(key, field)` | `&str, &str` | `Result<Option<String>>` | 获取字段值（UTF-8） |
| `hset(key, field, value)` | `&str, &str, V` | `Result<bool>` | 设置字段（返回是否新增） |
| `hdel(key, field)` | `&str, &str` | `Result<bool>` | 删除字段 |
| `hgetall(key)` | `&str` | `Result<HashMap<String, Vec<u8>>>` | 获取所有字段 |
| `hincrby(key, field, delta)` | `&str, &str, i64` | `Result<i64>` | 字段原子自增 |
| `hexists(key, field)` | `&str, &str` | `Result<bool>` | 字段是否存在 |
| `hkeys(key)` | `&str` | `Result<Vec<String>>` | 获取所有字段名 |
| `hvals(key)` | `&str` | `Result<Vec<Vec<u8>>>` | 获取所有字段值 |
| `hlen(key)` | `&str` | `Result<usize>` | 字段数量 |

### List 操作

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `lpush(key, value)` | `&str, V` | `Result<usize>` | 从左侧推入，返回长度 |
| `rpush(key, value)` | `&str, V` | `Result<usize>` | 从右侧推入，返回长度 |
| `lpop(key)` | `&str` | `Result<Option<Vec<u8>>>` | 从左侧弹出 |
| `rpop(key)` | `&str` | `Result<Option<Vec<u8>>>` | 从右侧弹出 |
| `lrange(key, start, stop)` | `&str, i64, i64` | `Result<Vec<Vec<u8>>>` | 获取范围（支持负索引） |
| `llen(key)` | `&str` | `Result<usize>` | 列表长度 |
| `lindex(key, index)` | `&str, i64` | `Result<Option<Vec<u8>>>` | 按索引获取（支持负索引） |

### Set 操作

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `sadd(key, member)` | `&str, V` | `Result<bool>` | 添加成员 |
| `srem(key, member)` | `&str, V` | `Result<bool>` | 移除成员 |
| `sismember(key, member)` | `&str, V` | `Result<bool>` | 成员是否存在 |
| `smembers(key)` | `&str` | `Result<Vec<Vec<u8>>>` | 获取所有成员 |
| `scard(key)` | `&str` | `Result<usize>` | 成员数量 |

### ZSet 操作

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `zadd(key, score, member)` | `&str, f64, &str` | `Result<bool>` | 添加/更新成员 |
| `zrem(key, member)` | `&str, &str` | `Result<bool>` | 移除成员 |
| `zscore(key, member)` | `&str, &str` | `Result<Option<f64>>` | 获取成员分数 |
| `zcard(key)` | `&str` | `Result<usize>` | 成员数量 |
| `zrange(key, start, stop)` | `&str, i64, i64` | `Result<Vec<(String, f64)>>` | 按索引范围获取 |
| `zrangebyscore(key, min, max)` | `&str, f64, f64` | `Result<Vec<(String, f64)>>` | 按分数范围获取 |
| `zincrby(key, delta, member)` | `&str, f64, &str` | `Result<f64>` | 成员分数自增 |

## TTL 过期

所有 `set` 操作都支持可选的 `ttl` 参数：

```rust
use std::time::Duration;

// 60 秒后自动过期
kv.set("session", b"abc".to_vec(), Some(Duration::from_secs(60))).await?;

// 永不过期（默认）
kv.set("config", b"{}".to_vec(), None).await?;

// 动态设置过期时间
kv.expire("session", Duration::from_secs(300)).await?;

// 查询剩余时间
if let Some(remaining) = kv.ttl("session").await? {
    println!("剩余 {} 秒", remaining.as_secs());
}
```

过期采用**惰性删除**策略：访问时检查是否过期，过期则自动清除。

## 线程安全

`MemKV` 内部使用 `tokio::sync::RwLock` 保护数据，可安全地在多个 async task 中共享：

```rust
let kv = MemKV::new();

// Clone 后可在多个 task 中使用
let kv1 = kv.clone();
let kv2 = kv.clone();

tokio::spawn(async move { kv1.set("a", b"1".to_vec(), None).await; });
tokio::spawn(async move { kv2.set("b", b"2".to_vec(), None).await; });
```

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 42901 | Type mismatch | 类型不匹配（如对 List key 执行 HGET） |
| 42902 | Key not found | Key 不存在 |
| 42903 | Index out of range | 索引越界 |
| 42904 | Value is not a number | 值不是有效数字（INCR 目标） |
| 52901 | Command failed | 操作执行失败 |

## 与 Redis 对照

| Redis 命令 | MemKV 方法 | 差异 |
|-----------|-----------|------|
| `GET` | `get()` / `get_str()` | 返回 `Vec<u8>` 或 `String` |
| `SET` | `set()` | TTL 用 `Duration` 而非秒数 |
| `SET NX` | `set_nx()` | — |
| `DEL` | `del()` | — |
| `EXISTS` | `exists()` | — |
| `EXPIRE` | `expire()` | 用 `Duration` |
| `TTL` | `ttl()` | 返回 `Option<Duration>` |
| `KEYS` | `keys()` | 支持 `*` 前缀/后缀通配 |
| `INCR` | `incr()` | — |
| `HGET/HSET/...` | `hget()/hset()/...` | — |
| `LPUSH/RPOP/...` | `lpush()/rpop()/...` | — |
| `SADD/SMEMBERS/...` | `sadd()/smembers()/...` | — |
| `ZADD/ZRANGE/...` | `zadd()/zrange()/...` | — |
| `SCAN` | `keys()` | 简化实现，生产环境慎用 |
| `SUBSCRIBE` | ❌ | 暂不支持 |
| `MULTI/EXEC` | ❌ | 暂不支持 |
