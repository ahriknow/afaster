# Bloom Filter 布隆过滤器

> Feature: `bloom`

基于内存的概率型数据结构，用于快速判断元素是否"可能存在"或"一定不存在"。结合 Hook 机制，可在请求处理前自动检查。

## 应用场景

- **请求去重**：防止同一请求被重复处理
- **IP 黑名单**：快速过滤已知恶意 IP
- **爬虫检测**：识别已访问的 User-Agent
- **缓存穿透防护**：过滤不存在的 key

## 配置

```toml
[bloom]
expected_items = 100000     # 预期存储的元素数量, 默认 100000
false_positive_rate = 0.01  # 可接受的误判率 (0.0~1.0), 默认 0.01
auto_check_key = "ip"       # Hook 自动检查的请求属性（见下表）
```

### `auto_check_key` 可选值

| 值 | 说明 |
|---|---|
| `"ip"` | 优先真实 IP (`X-Forwarded-For` / `X-Real-IP`)，无代理时回退到代理 IP (TCP 直连地址) |
| `"forwarded_for"` | 真实客户端 IP (`X-Forwarded-For` / `X-Real-IP`)，无代理时回退到代理 IP |
| `"client_ip"` | 代理 IP（TCP 直连地址） |
| `"path"` | 请求路径（handler 名称） |
| `"method+path"` | HTTP 方法 + 路径 |
| `"handler"` | handler 名称 |
| `"header:<name>"` | 指定 Header 值 |
| 不设置 | 仅将 `BloomFilter` 写入 `ctx`，由 handler 手动使用 |

> **IP 说明**：`forwarded_for` 来自反向代理（Nginx、CDN 等）传递的 `X-Forwarded-For` 或 `X-Real-IP` 头，是真实客户端 IP；`client_ip` 是 TCP 连接的直连地址，在有代理时为代理服务器 IP。

## 使用

### 自动检查模式

配置 `auto_check_key` 后，Hook 会自动提取请求 key 并检查布隆过滤器，结果通过 `Ctx<BloomFilterContext>` 获取：

```rust
use afaster::bloom::{BloomFilter, BloomFilterContext};
use afast::Ctx;

async fn my_handler(
    Ctx(ctx): Ctx<BloomFilterContext>,
) -> HttpResult<Json<serde_json::Value>> {
    if ctx.exists {
        return Err(afaster::Error::custom(41001, "Duplicate request"));
    }
    // 处理请求...
    Ok(Json(serde_json::json!({ "ok": true })))
}
```

### 手动模式

不设置 `auto_check_key` 时，可通过 `Ctx<BloomFilter>` 手动操作：

```rust
use afaster::bloom::BloomFilter;
use afast::Ctx;

async fn my_handler(
    Ctx(bloom): Ctx<BloomFilter>,
) -> HttpResult<Json<serde_json::Value>> {
    let key = "some_unique_key";
    if bloom.check(key) {
        return Err(afaster::Error::custom(41001, "Duplicate request"));
    }
    bloom.add(key);
    Ok(Json(serde_json::json!({ "ok": true })))
}
```

## API

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `new(expected_items, false_positive_rate)` | `usize, f64` | `BloomFilter` | 创建实例 |
| `from_config(config)` | `&BloomFilterConfig` | `BloomFilter` | 从配置创建 |
| `add(item)` | `&str` | `()` | 添加元素 |
| `check(item)` | `&str` | `bool` | 检查元素是否可能存在 |
| `check_and_add(item)` | `&str` | `bool` | 原子检查并添加，返回是否已存在 |
| `clear()` | — | `()` | 重置过滤器 |
| `num_hashes()` | — | `usize` | 哈希函数数量 (k) |
| `num_bits()` | — | `usize` | 位数组大小 (m) |
| `expected_items()` | — | `usize` | 预期容量 (n) |
| `false_positive_rate()` | — | `f64` | 配置的误判率 |

## BloomFilterContext

Hook 自动检查时写入 `ctx` 的上下文：

| 字段 | 类型 | 说明 |
|------|------|------|
| `key` | `String` | 从请求中提取的 key |
| `exists` | `bool` | 该 key 是否已存在于布隆过滤器中 |

## 实现细节

- 线程安全，内部使用 `RwLock` 保护位数组
- 双重哈希（FNV-1a）+ Kirsch-Mitzenmacher 优化，仅需两次哈希计算生成 k 个位置
- 最优位数组大小：$m = -\frac{n \ln p}{(\ln 2)^2}$
- 最优哈希函数数量：$k = \frac{m}{n} \ln 2$
