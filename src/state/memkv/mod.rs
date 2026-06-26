//! 内存 KV 数据库
//!
//! 支持与 Redis 兼容的数据类型：String / Hash / List / Set / ZSet。
//! 纯内存实现，暂无持久化（计划通过 ahrifs 实现）。
//!
//! # 使用
//!
//! ```ignore
//! let kv = MemKV::new();
//! kv.set("key", "value", None).await?;
//! let v: Option<String> = kv.get("key").await?;
//! ```

pub mod err;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ═══════════════════════════════════════════════════════════════
//  内部值类型
// ═══════════════════════════════════════════════════════════════

#[derive(Clone, Debug)]
enum Value {
    /// STRING: bytes
    String(Vec<u8>),
    /// HASH: field → value
    Hash(HashMap<String, Vec<u8>>),
    /// List: 有序数组
    List(Vec<Vec<u8>>),
    /// Set: 无序集合
    Set(HashSet<Vec<u8>>),
    /// Sorted Set: score → member（BTreeMap 保证有序）
    ZSet(BTreeMap<String, ZSetEntry>),
}

#[derive(Clone, Debug, PartialEq)]
struct ZSetEntry {
    member: String,
    score: f64,
}

/// BTreeMap 排序 key：`(score, member)` 以保证按分数排序
impl Eq for ZSetEntry {}

impl PartialOrd for ZSetEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ZSetEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| self.member.cmp(&other.member))
    }
}

/// 带过期时间的值
#[derive(Clone, Debug)]
struct Entry {
    value: Value,
    expires_at: Option<Instant>,
}

impl Entry {
    fn new(value: Value, ttl: Option<Duration>) -> Self {
        Self {
            value,
            expires_at: ttl.map(|d| Instant::now() + d),
        }
    }

    fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|t| Instant::now() >= t)
    }
}

// ═══════════════════════════════════════════════════════════════
//  MemKV
// ═══════════════════════════════════════════════════════════════

/// 内存 KV 数据库
///
/// 线程安全，内部使用 `RwLock` 保护。
/// 支持 String / Hash / List / Set / ZSet 五种数据类型。
#[derive(Clone)]
pub struct MemKV {
    inner: std::sync::Arc<RwLock<HashMap<String, Entry>>>,
}

impl MemKV {
    /// 创建新的 MemKV 实例
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // ══════════════════════════════════════════════════════════
    //  STRING 操作
    // ══════════════════════════════════════════════════════════

    /// GET — 获取字符串值
    pub async fn get(&self, key: &str) -> crate::Result<Option<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            match &entry.value {
                Value::String(v) => Ok(Some(v.clone())),
                _ => Err(err::type_mismatch("string", entry.value.type_name())),
            }
        } else {
            Ok(None)
        }
    }

    /// GET — 获取字符串值并转为 UTF-8
    pub async fn get_str(&self, key: &str) -> crate::Result<Option<String>> {
        match self.get(key).await? {
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| err::cmd_failed("GET", &format!("invalid utf8: {}", e))),
            None => Ok(None),
        }
    }

    /// SET — 设置字符串值
    ///
    /// - `ttl` 为过期时间，`None` 表示永不过期
    pub async fn set<V: Into<Vec<u8>>>(
        &self,
        key: &str,
        value: V,
        ttl: Option<Duration>,
    ) -> crate::Result<()> {
        let mut map = self.inner.write().await;
        map.insert(
            key.to_string(),
            Entry::new(Value::String(value.into()), ttl),
        );
        Ok(())
    }

    /// SET NX — 仅当 key 不存在时设置
    pub async fn set_nx<V: Into<Vec<u8>>>(
        &self,
        key: &str,
        value: V,
        ttl: Option<Duration>,
    ) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key)
            && !entry.is_expired()
        {
            return Ok(false);
        }
        map.insert(
            key.to_string(),
            Entry::new(Value::String(value.into()), ttl),
        );
        Ok(true)
    }

    /// DEL — 删除 key
    pub async fn del(&self, key: &str) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        Ok(map.remove(key).is_some())
    }

    /// EXISTS — key 是否存在
    pub async fn exists(&self, key: &str) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// EXPIRE — 设置过期时间
    pub async fn expire(&self, key: &str, ttl: Duration) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            entry.expires_at = Some(Instant::now() + ttl);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// TTL — 获取剩余过期时间
    ///
    /// - `Some(duration)` — 剩余时间
    /// - `None` — 永不过期
    /// - key 不存在返回 `Err`
    pub async fn ttl(&self, key: &str) -> crate::Result<Option<Duration>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Err(err::key_not_found(key));
            }
            Ok(entry.expires_at.map(|t| {
                let now = Instant::now();
                if t > now { t - now } else { Duration::ZERO }
            }))
        } else {
            Err(err::key_not_found(key))
        }
    }

    /// KEYS — 按前缀查找 key（简单实现，生产环境慎用）
    pub async fn keys(&self, pattern: &str) -> crate::Result<Vec<String>> {
        let map = self.inner.read().await;
        let now = Instant::now();
        let result: Vec<String> = map
            .iter()
            .filter(|(_, entry)| entry.expires_at.is_none_or(|t| now < t))
            .filter(|(k, _)| {
                if pattern == "*" {
                    true
                } else if let Some(prefix) = pattern.strip_suffix('*') {
                    k.starts_with(prefix)
                } else if let Some(suffix) = pattern.strip_prefix('*') {
                    k.ends_with(suffix)
                } else {
                    k.as_str() == pattern
                }
            })
            .map(|(k, _)| k.clone())
            .collect();
        Ok(result)
    }

    /// INCR — 原子自增
    pub async fn incr(&self, key: &str, delta: i64) -> crate::Result<i64> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                let val = delta;
                map.insert(
                    key.to_string(),
                    Entry::new(Value::String(val.to_string().into()), None),
                );
                return Ok(val);
            }
            match &mut entry.value {
                Value::String(bytes) => {
                    let s = String::from_utf8(bytes.clone()).map_err(|_| err::not_a_number(key))?;
                    let current: i64 = s.parse().map_err(|_| err::not_a_number(&s))?;
                    let new_val = current + delta;
                    *bytes = new_val.to_string().into_bytes();
                    Ok(new_val)
                }
                _ => Err(err::type_mismatch("string", entry.value.type_name())),
            }
        } else {
            let val = delta;
            map.insert(
                key.to_string(),
                Entry::new(Value::String(val.to_string().into()), None),
            );
            Ok(val)
        }
    }

    // ══════════════════════════════════════════════════════════
    //  HASH 操作
    // ══════════════════════════════════════════════════════════

    /// HGET — 获取 hash 字段值
    pub async fn hget(&self, key: &str, field: &str) -> crate::Result<Option<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            match &entry.value {
                Value::Hash(h) => Ok(h.get(field).cloned()),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(None)
        }
    }

    /// HGET — 获取 hash 字段值并转为 UTF-8
    pub async fn hget_str(&self, key: &str, field: &str) -> crate::Result<Option<String>> {
        match self.hget(key, field).await? {
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|e| err::cmd_failed("HGET", &format!("invalid utf8: {}", e))),
            None => Ok(None),
        }
    }

    /// HSET — 设置 hash 字段
    pub async fn hset<V: Into<Vec<u8>>>(
        &self,
        key: &str,
        field: &str,
        value: V,
    ) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::Hash(HashMap::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::Hash(HashMap::new()), None);
        }
        match &mut entry.value {
            Value::Hash(h) => {
                let is_new = !h.contains_key(field);
                h.insert(field.to_string(), value.into());
                Ok(is_new)
            }
            _ => Err(err::type_mismatch("hash", entry.value.type_name())),
        }
    }

    /// HDEL — 删除 hash 字段
    pub async fn hdel(&self, key: &str, field: &str) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            match &mut entry.value {
                Value::Hash(h) => Ok(h.remove(field).is_some()),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(false)
        }
    }

    /// HGETALL — 获取所有 hash 字段和值
    pub async fn hgetall(&self, key: &str) -> crate::Result<HashMap<String, Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(HashMap::new());
            }
            match &entry.value {
                Value::Hash(h) => Ok(h.clone()),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(HashMap::new())
        }
    }

    /// HINCRBY — hash 字段原子自增
    pub async fn hincrby(&self, key: &str, field: &str, delta: i64) -> crate::Result<i64> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::Hash(HashMap::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::Hash(HashMap::new()), None);
        }
        match &mut entry.value {
            Value::Hash(h) => {
                let current = if let Some(bytes) = h.get(field) {
                    let s =
                        String::from_utf8(bytes.clone()).map_err(|_| err::not_a_number(field))?;
                    s.parse::<i64>().map_err(|_| err::not_a_number(&s))?
                } else {
                    0
                };
                let new_val = current + delta;
                h.insert(field.to_string(), new_val.to_string().into_bytes());
                Ok(new_val)
            }
            _ => Err(err::type_mismatch("hash", entry.value.type_name())),
        }
    }

    /// HEXISTS — hash 字段是否存在
    pub async fn hexists(&self, key: &str, field: &str) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            match &entry.value {
                Value::Hash(h) => Ok(h.contains_key(field)),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(false)
        }
    }

    /// HKEYS — 获取所有 hash 字段名
    pub async fn hkeys(&self, key: &str) -> crate::Result<Vec<String>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(vec![]);
            }
            match &entry.value {
                Value::Hash(h) => Ok(h.keys().cloned().collect()),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// HVALS — 获取所有 hash 字段值
    pub async fn hvals(&self, key: &str) -> crate::Result<Vec<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(vec![]);
            }
            match &entry.value {
                Value::Hash(h) => Ok(h.values().cloned().collect()),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// HLEN — hash 字段数量
    pub async fn hlen(&self, key: &str) -> crate::Result<usize> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(0);
            }
            match &entry.value {
                Value::Hash(h) => Ok(h.len()),
                _ => Err(err::type_mismatch("hash", entry.value.type_name())),
            }
        } else {
            Ok(0)
        }
    }

    // ══════════════════════════════════════════════════════════
    //  List 操作
    // ══════════════════════════════════════════════════════════

    /// LPUSH — 从左侧推入
    pub async fn lpush<V: Into<Vec<u8>>>(&self, key: &str, value: V) -> crate::Result<usize> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::List(Vec::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::List(Vec::new()), None);
        }
        match &mut entry.value {
            Value::List(l) => {
                l.insert(0, value.into());
                Ok(l.len())
            }
            _ => Err(err::type_mismatch("list", entry.value.type_name())),
        }
    }

    /// RPUSH — 从右侧推入
    pub async fn rpush<V: Into<Vec<u8>>>(&self, key: &str, value: V) -> crate::Result<usize> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::List(Vec::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::List(Vec::new()), None);
        }
        match &mut entry.value {
            Value::List(l) => {
                l.push(value.into());
                Ok(l.len())
            }
            _ => Err(err::type_mismatch("list", entry.value.type_name())),
        }
    }

    /// LPOP — 从左侧弹出
    pub async fn lpop(&self, key: &str) -> crate::Result<Option<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            match &mut entry.value {
                Value::List(l) => {
                    if l.is_empty() {
                        Ok(None)
                    } else {
                        Ok(Some(l.remove(0)))
                    }
                }
                _ => Err(err::type_mismatch("list", entry.value.type_name())),
            }
        } else {
            Ok(None)
        }
    }

    /// RPOP — 从右侧弹出
    pub async fn rpop(&self, key: &str) -> crate::Result<Option<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            match &mut entry.value {
                Value::List(l) => Ok(l.pop()),
                _ => Err(err::type_mismatch("list", entry.value.type_name())),
            }
        } else {
            Ok(None)
        }
    }

    /// LRANGE — 获取列表范围（支持负索引，-1 表示最后一个）
    pub async fn lrange(&self, key: &str, start: i64, stop: i64) -> crate::Result<Vec<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(vec![]);
            }
            match &entry.value {
                Value::List(l) => {
                    let len = l.len() as i64;
                    let s = normalize_index(start, len);
                    let e = normalize_index(stop, len);
                    if s >= len || s > e {
                        return Ok(vec![]);
                    }
                    let e = (e + 1).min(len) as usize;
                    Ok(l[s as usize..e].to_vec())
                }
                _ => Err(err::type_mismatch("list", entry.value.type_name())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// LLEN — 列表长度
    pub async fn llen(&self, key: &str) -> crate::Result<usize> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(0);
            }
            match &entry.value {
                Value::List(l) => Ok(l.len()),
                _ => Err(err::type_mismatch("list", entry.value.type_name())),
            }
        } else {
            Ok(0)
        }
    }

    /// LINDEX — 按索引获取元素（支持负索引）
    pub async fn lindex(&self, key: &str, index: i64) -> crate::Result<Option<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            match &entry.value {
                Value::List(l) => {
                    let len = l.len() as i64;
                    let i = normalize_index(index, len);
                    if i < 0 || i >= len {
                        Ok(None)
                    } else {
                        Ok(Some(l[i as usize].clone()))
                    }
                }
                _ => Err(err::type_mismatch("list", entry.value.type_name())),
            }
        } else {
            Ok(None)
        }
    }

    // ══════════════════════════════════════════════════════════
    //  Set 操作
    // ══════════════════════════════════════════════════════════

    /// SADD — 添加成员
    pub async fn sadd<V: Into<Vec<u8>>>(&self, key: &str, member: V) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::Set(HashSet::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::Set(HashSet::new()), None);
        }
        match &mut entry.value {
            Value::Set(s) => Ok(s.insert(member.into())),
            _ => Err(err::type_mismatch("set", entry.value.type_name())),
        }
    }

    /// SREM — 移除成员
    pub async fn srem<V: Into<Vec<u8>>>(&self, key: &str, member: V) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            match &mut entry.value {
                Value::Set(s) => Ok(s.remove(&member.into())),
                _ => Err(err::type_mismatch("set", entry.value.type_name())),
            }
        } else {
            Ok(false)
        }
    }

    /// SISMEMBER — 成员是否存在
    pub async fn sismember<V: Into<Vec<u8>>>(&self, key: &str, member: V) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            match &entry.value {
                Value::Set(s) => Ok(s.contains(&member.into())),
                _ => Err(err::type_mismatch("set", entry.value.type_name())),
            }
        } else {
            Ok(false)
        }
    }

    /// SMEMBERS — 获取所有成员
    pub async fn smembers(&self, key: &str) -> crate::Result<Vec<Vec<u8>>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(vec![]);
            }
            match &entry.value {
                Value::Set(s) => Ok(s.iter().cloned().collect()),
                _ => Err(err::type_mismatch("set", entry.value.type_name())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// SCARD — 集合成员数量
    pub async fn scard(&self, key: &str) -> crate::Result<usize> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(0);
            }
            match &entry.value {
                Value::Set(s) => Ok(s.len()),
                _ => Err(err::type_mismatch("set", entry.value.type_name())),
            }
        } else {
            Ok(0)
        }
    }

    // ══════════════════════════════════════════════════════════
    //  Sorted Set 操作
    // ══════════════════════════════════════════════════════════

    /// ZADD — 添加成员（已存在则更新 score）
    pub async fn zadd(&self, key: &str, score: f64, member: &str) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::ZSet(BTreeMap::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::ZSet(BTreeMap::new()), None);
        }
        match &mut entry.value {
            Value::ZSet(z) => {
                let is_new = !z.contains_key(member);
                z.insert(
                    member.to_string(),
                    ZSetEntry {
                        member: member.to_string(),
                        score,
                    },
                );
                Ok(is_new)
            }
            _ => Err(err::type_mismatch("zset", entry.value.type_name())),
        }
    }

    /// ZREM — 移除成员
    pub async fn zrem(&self, key: &str, member: &str) -> crate::Result<bool> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get_mut(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(false);
            }
            match &mut entry.value {
                Value::ZSet(z) => Ok(z.remove(member).is_some()),
                _ => Err(err::type_mismatch("zset", entry.value.type_name())),
            }
        } else {
            Ok(false)
        }
    }

    /// ZSCORE — 获取成员分数
    pub async fn zscore(&self, key: &str, member: &str) -> crate::Result<Option<f64>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            match &entry.value {
                Value::ZSet(z) => Ok(z.get(member).map(|e| e.score)),
                _ => Err(err::type_mismatch("zset", entry.value.type_name())),
            }
        } else {
            Ok(None)
        }
    }

    /// ZCARD — 有序集合成员数量
    pub async fn zcard(&self, key: &str) -> crate::Result<usize> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(0);
            }
            match &entry.value {
                Value::ZSet(z) => Ok(z.len()),
                _ => Err(err::type_mismatch("zset", entry.value.type_name())),
            }
        } else {
            Ok(0)
        }
    }

    /// ZRANGE — 按索引范围获取成员（分数从低到高）
    pub async fn zrange(
        &self,
        key: &str,
        start: i64,
        stop: i64,
    ) -> crate::Result<Vec<(String, f64)>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(vec![]);
            }
            match &entry.value {
                Value::ZSet(z) => {
                    let len = z.len() as i64;
                    let s = normalize_index(start, len);
                    let e = normalize_index(stop, len);
                    if s >= len || s > e {
                        return Ok(vec![]);
                    }
                    let e = (e + 1).min(len) as usize;
                    Ok(z.values()
                        .skip(s as usize)
                        .take(e - s as usize)
                        .map(|entry| (entry.member.clone(), entry.score))
                        .collect())
                }
                _ => Err(err::type_mismatch("zset", entry.value.type_name())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// ZRANGEBYSCORE — 按分数范围获取成员
    pub async fn zrangebyscore(
        &self,
        key: &str,
        min: f64,
        max: f64,
    ) -> crate::Result<Vec<(String, f64)>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(vec![]);
            }
            match &entry.value {
                Value::ZSet(z) => Ok(z
                    .values()
                    .filter(|e| e.score >= min && e.score <= max)
                    .map(|e| (e.member.clone(), e.score))
                    .collect()),
                _ => Err(err::type_mismatch("zset", entry.value.type_name())),
            }
        } else {
            Ok(vec![])
        }
    }

    /// ZINCRBY — 成员分数原子自增
    pub async fn zincrby(&self, key: &str, delta: f64, member: &str) -> crate::Result<f64> {
        let mut map = self.inner.write().await;
        let entry = map
            .entry(key.to_string())
            .or_insert_with(|| Entry::new(Value::ZSet(BTreeMap::new()), None));
        if entry.is_expired() {
            *entry = Entry::new(Value::ZSet(BTreeMap::new()), None);
        }
        match &mut entry.value {
            Value::ZSet(z) => {
                let new_score = if let Some(e) = z.get(member) {
                    e.score + delta
                } else {
                    delta
                };
                z.insert(
                    member.to_string(),
                    ZSetEntry {
                        member: member.to_string(),
                        score: new_score,
                    },
                );
                Ok(new_score)
            }
            _ => Err(err::type_mismatch("zset", entry.value.type_name())),
        }
    }

    // ══════════════════════════════════════════════════════════
    //  通用操作
    // ══════════════════════════════════════════════════════════

    /// TYPE — 获取 key 的类型
    pub async fn typ(&self, key: &str) -> crate::Result<Option<&'static str>> {
        let mut map = self.inner.write().await;
        if let Some(entry) = map.get(key) {
            if entry.is_expired() {
                map.remove(key);
                return Ok(None);
            }
            Ok(Some(entry.value.type_name()))
        } else {
            Ok(None)
        }
    }

    /// DBSIZE — key 总数（不含已过期）
    pub async fn dbsize(&self) -> crate::Result<usize> {
        let map = self.inner.read().await;
        let now = Instant::now();
        Ok(map
            .values()
            .filter(|e| e.expires_at.is_none_or(|t| now < t))
            .count())
    }

    /// FLUSHDB — 清空所有数据
    pub async fn flushdb(&self) -> crate::Result<()> {
        let mut map = self.inner.write().await;
        map.clear();
        Ok(())
    }
}

impl Default for MemKV {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════
//  辅助函数
// ═══════════════════════════════════════════════════════════════

/// 处理负索引（-1 = 最后一个，-2 = 倒数第二个...）
fn normalize_index(index: i64, len: i64) -> i64 {
    if index < 0 {
        (len + index).max(0)
    } else {
        index
    }
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Hash(_) => "hash",
            Value::List(_) => "list",
            Value::Set(_) => "set",
            Value::ZSet(_) => "zset",
        }
    }
}
