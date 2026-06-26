//! # Bloom Filter 布隆过滤器
//!
//! 基于内存的概率型数据结构，用于快速判断元素是否"可能存在"或"一定不存在"。
//! 结合 Hook 机制，可在请求处理前自动检查，适用于：
//!
//! - **请求去重**：防止同一请求被重复处理
//! - **IP 黑名单**：快速过滤已知恶意 IP
//! - **爬虫检测**：识别已访问的 User-Agent
//! - **缓存穿透防护**：过滤不存在的 key
//!
//! # Feature
//!
//! 启用 `bloom` 后自动生效：
//! - Hook 自动将 `BloomFilter` 写入 `ctx.ctx`，handler 通过 `Ctx<BloomFilter>` 提取
//! - 支持配置 `auto_check_key`，自动检查请求 IP / Path 等
//! - 可配置误判率和预期容量
//!
//! # 配置
//!
//! ```toml
//! [bloom]
//! expected_items = 100_000
//! false_positive_rate = 0.01
//! auto_check_key = "ip"             # 优先真实 IP (forwarded_for)，回退代理 IP (client_ip)
//! # auto_check_key = "forwarded_for"  # 仅真实 IP (X-Forwarded-For / X-Real-IP)，回退代理 IP
//! # auto_check_key = "client_ip"     # 仅代理 IP (TCP 直连地址)
//! ```
//!
//! # 使用示例
//!
//! ```ignore
//! use afaster::bloom::BloomFilter;
//! use afast::Ctx;
//!
//! async fn my_handler(Ctx(bloom): Ctx<BloomFilter>) -> HttpResult<Json<serde_json::Value>> {
//!     let key = "some_unique_key";
//!     if bloom.check(key) {
//!         return Err(afaster::Error::custom(41001, "Duplicate request"));
//!     }
//!     bloom.add(key);
//!     Ok(Json(serde_json::json!({ "ok": true })))
//! }
//! ```

use serde::Deserialize;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

fn default_expected_items() -> usize {
    100_000
}

fn default_false_positive_rate() -> f64 {
    0.01
}

/// 布隆过滤器配置（对应 config.toml 的 `[bloom]`）
#[derive(Debug, Clone, Deserialize)]
pub struct BloomFilterConfig {
    /// 预期存储的元素数量
    #[serde(default = "default_expected_items")]
    pub expected_items: usize,
    /// 可接受的误判率 (0.0 ~ 1.0)
    #[serde(default = "default_false_positive_rate")]
    pub false_positive_rate: f64,
    /// Hook 自动检查的请求属性
    ///
    /// - `"ip"`: 优先真实 IP (`forwarded_for`)，回退代理 IP (`client_ip`)
    /// - `"forwarded_for"`: 真实客户端 IP (`X-Forwarded-For` / `X-Real-IP`)，无代理时回退到 `client_ip`
    /// - `"client_ip"`: 代理 IP（TCP 直连地址）
    /// - `"path"`: 请求路径
    /// - `"method+path"`: HTTP 方法 + 路径
    /// - `"header:<name>"`: 指定 Header 值
    /// - `"handler"`: handler 名称
    /// - `null` / 未设置: 不自动检查，仅将 BloomFilter 写入 ctx
    #[serde(default)]
    pub auto_check_key: Option<String>,
}

impl Default for BloomFilterConfig {
    fn default() -> Self {
        Self {
            expected_items: default_expected_items(),
            false_positive_rate: default_false_positive_rate(),
            auto_check_key: None,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  布隆过滤器核心
// ═══════════════════════════════════════════════════════════════

/// 布隆过滤器
///
/// 线程安全，内部使用 `RwLock` 保护位数组。
/// 采用双重哈希（FNV-1a）+ Kirsch-Mitzenmacher 优化，高效计算 k 个哈希位置。
///
/// # 示例
///
/// ```ignore
/// let filter = BloomFilter::new(100_000, 0.01);
/// filter.add("hello");
/// assert!(filter.check("hello"));
/// assert!(!filter.check("world")); // 可能误判，概率约 1%
/// ```
#[derive(Clone)]
pub struct BloomFilter {
    bits: std::sync::Arc<RwLock<Vec<u64>>>,
    num_hashes: usize,
    num_bits: usize,
    config: BloomFilterConfig,
}

impl BloomFilter {
    /// 创建新的布隆过滤器
    ///
    /// - `expected_items`: 预期存储的元素数量
    /// - `false_positive_rate`: 可接受的误判率 (0.0 ~ 1.0)
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        let num_bits = Self::optimal_bits(expected_items, false_positive_rate);
        let num_hashes = Self::optimal_hashes(num_bits, expected_items);

        Self {
            bits: std::sync::Arc::new(RwLock::new(vec![0u64; num_bits.div_ceil(64)])),
            num_hashes,
            num_bits,
            config: BloomFilterConfig {
                expected_items,
                false_positive_rate,
                auto_check_key: None,
            },
        }
    }

    /// 从配置创建布隆过滤器
    pub fn from_config(config: &BloomFilterConfig) -> Self {
        let mut bf = Self::new(config.expected_items, config.false_positive_rate);
        bf.config = config.clone();
        bf
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let config: BloomFilterConfig = match table.get("bloom") {
            Some(v) => v
                .clone()
                .try_into()
                .map_err(|e| crate::Error::custom(50001, format!("[bloom] {}", e)))?,
            None => BloomFilterConfig::default(),
        };
        Ok(Self::from_config(&config))
    }

    /// 向布隆过滤器添加元素
    ///
    /// 线程安全，使用写锁。
    pub fn add(&self, item: &str) {
        let positions = self.hash_positions(item);
        let mut bits = self.bits.write().unwrap();
        for pos in positions {
            let index = pos / 64;
            let offset = pos % 64;
            if index < bits.len() {
                bits[index] |= 1u64 << offset;
            }
        }
    }

    /// 检查元素是否**可能**存在于布隆过滤器中
    ///
    /// - 返回 `true`：元素**可能存在**（有误判可能）
    /// - 返回 `false`：元素**一定不存在**（100% 准确）
    pub fn check(&self, item: &str) -> bool {
        let positions = self.hash_positions(item);
        let bits = self.bits.read().unwrap();
        positions.iter().all(|&pos| {
            let index = pos / 64;
            let offset = pos % 64;
            index < bits.len() && (bits[index] & (1u64 << offset)) != 0
        })
    }

    /// 原子性地检查并添加：如果元素不存在则添加，返回元素是否已存在
    ///
    /// - 返回 `true`：元素**已存在**（之前添加过，或误判）
    /// - 返回 `false`：元素**不存在**，已成功添加
    pub fn check_and_add(&self, item: &str) -> bool {
        let positions = self.hash_positions(item);
        let mut bits = self.bits.write().unwrap();

        let mut exists = true;
        for &pos in &positions {
            let index = pos / 64;
            let offset = pos % 64;
            if index < bits.len() && (bits[index] & (1u64 << offset)) == 0 {
                exists = false;
                bits[index] |= 1u64 << offset;
            }
        }
        exists
    }

    /// 重置布隆过滤器（清空所有位）
    pub fn clear(&self) {
        let mut bits = self.bits.write().unwrap();
        bits.fill(0);
    }

    // ── 元数据 ──

    /// 获取哈希函数数量 (k)
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }

    /// 获取位数组大小 (m)
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// 获取预期容量 (n)
    pub fn expected_items(&self) -> usize {
        self.config.expected_items
    }

    /// 获取配置的误判率
    pub fn false_positive_rate(&self) -> f64 {
        self.config.false_positive_rate
    }

    /// 获取当前配置
    pub fn config(&self) -> &BloomFilterConfig {
        &self.config
    }

    // ── 内部实现 ──

    /// 计算元素的 k 个哈希位置
    ///
    /// 使用双重哈希 + Kirsch-Mitzenmacher 优化：
    /// `h(i) = h1 + i * h2`，仅需两次哈希计算即可生成 k 个位置。
    fn hash_positions(&self, item: &str) -> Vec<usize> {
        let h1 = self.fnv_hash(item, 0);
        let h2 = self.fnv_hash(item, h1.wrapping_add(1));

        (0..self.num_hashes)
            .map(|i| {
                let hash = h1.wrapping_add((i as u64).wrapping_mul(h2));
                (hash as usize) % self.num_bits
            })
            .collect()
    }

    /// FNV-1a 哈希实现
    ///
    /// 使用不同的 seed 产生两个独立的哈希值。
    fn fnv_hash(&self, item: &str, seed: u64) -> u64 {
        let mut hasher = FnvHasher::with_seed(seed);
        item.hash(&mut hasher);
        hasher.finish()
    }

    /// 计算最优位数组大小: m = -(n * ln(p)) / (ln(2))^2
    fn optimal_bits(n: usize, p: f64) -> usize {
        let ln2_squared = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        (-(n as f64) * p.ln() / ln2_squared).ceil() as usize
    }

    /// 计算最优哈希函数数量: k = (m / n) * ln(2)
    fn optimal_hashes(m: usize, n: usize) -> usize {
        let k = (m as f64 / n as f64) * std::f64::consts::LN_2;
        (k.round() as usize).max(1)
    }
}

// ═══════════════════════════════════════════════════════════════
//  FNV-1a 哈希器
// ═══════════════════════════════════════════════════════════════

/// FNV-1a 64 位哈希器（支持自定义 seed）
struct FnvHasher {
    state: u64,
}

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001b3;

impl FnvHasher {
    fn with_seed(seed: u64) -> Self {
        Self {
            state: FNV_OFFSET_BASIS ^ seed,
        }
    }
}

impl Hasher for FnvHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

// ═══════════════════════════════════════════════════════════════
//  Per-Request Context
// ═══════════════════════════════════════════════════════════════

/// 布隆过滤器请求上下文
///
/// 由 Hook 在 `before_request` 中写入 `ctx.ctx`，
/// handler 通过 `Ctx<BloomFilterContext>` 提取。
#[derive(Clone)]
pub struct BloomFilterContext {
    /// 从请求中提取的 key
    pub key: String,
    /// 该 key 是否已存在于布隆过滤器中
    pub exists: bool,
}

// ═══════════════════════════════════════════════════════════════
//  Hook 集成
// ═══════════════════════════════════════════════════════════════

/// 从 BloomFilter 创建 Hook
///
/// 在 `run()` 中调用，state move 之前 clone BloomFilter。
pub fn create_hook(bf: &BloomFilter) -> BloomFilterHook {
    BloomFilterHook { filter: bf.clone() }
}

/// 布隆过滤器 Hook
///
/// 自动将 `BloomFilter` 写入 per-request `ctx.ctx`，
/// handler 可通过 `Ctx<BloomFilter>` 提取使用。
///
/// 如果配置了 `auto_check_key`，还会自动提取请求 key 并检查布隆过滤器，
/// 将结果写入 `BloomFilterContext`（`Ctx<BloomFilterContext>`）。
pub struct BloomFilterHook {
    filter: BloomFilter,
}

impl BloomFilterHook {
    /// 从请求上下文中提取 key
    fn extract_key(&self, ctx: &afast::hook::RequestContext) -> Option<String> {
        let key_cfg = self.filter.config.auto_check_key.as_deref()?;
        match key_cfg {
            "client_ip" => {
                // 代理 IP（TCP 直连地址）
                Some(format!("ip:{}", ctx.client_ip))
            }
            "forwarded_for" => {
                // 真实客户端 IP (X-Forwarded-For / X-Real-IP)，无代理时回退到代理 IP
                let ip = ctx.forwarded_for.as_deref().unwrap_or(&ctx.client_ip);
                Some(format!("ip:{}", ip))
            }
            "ip" => {
                // 优先真实 IP (forwarded_for)，回退代理 IP (client_ip)
                let ip = ctx.forwarded_for.as_deref().unwrap_or(&ctx.client_ip);
                Some(format!("ip:{}", ip))
            }
            "path" => Some(ctx.handler_name.to_string()),
            "method+path" => Some(format!("{}:{}", ctx.method, ctx.handler_name)),
            "handler" => Some(ctx.handler_name.to_string()),
            s if s.starts_with("header:") => {
                // Header 提取需要 request extensions，此处回退到 handler_name
                Some(format!("{}:{}", s, ctx.handler_name))
            }
            _ => None,
        }
    }
}

impl afast::hook::Hook for BloomFilterHook {
    fn before_request(
        &self,
        ctx: &afast::hook::RequestContext,
    ) -> Option<Box<dyn afast::hook::RequestGuard>> {
        // 将 BloomFilter 写入 per-request ctx，handler 可通过 Ctx<BloomFilter> 提取
        ctx.ctx.insert(self.filter.clone());

        // 自动检查模式
        if let Some(key) = self.extract_key(ctx) {
            let exists = {
                let bits = self.filter.bits.read().unwrap();
                let positions = self.filter.hash_positions(&key);
                positions.iter().all(|&pos| {
                    let index = pos / 64;
                    let offset = pos % 64;
                    index < bits.len() && (bits[index] & (1u64 << offset)) != 0
                })
            };

            ctx.ctx.insert(BloomFilterContext {
                key: key.clone(),
                exists,
            });

            // 自动将 key 添加到布隆过滤器（标记为已处理）
            {
                let mut bits = self.filter.bits.write().unwrap();
                let positions = self.filter.hash_positions(&key);
                for pos in positions {
                    let index = pos / 64;
                    let offset = pos % 64;
                    if index < bits.len() {
                        bits[index] |= 1u64 << offset;
                    }
                }
            }

            #[cfg(feature = "log")]
            if exists {
                ::tracing::warn!(
                    handler = ctx.handler_name,
                    key = %key,
                    "bloom: key already exists (possible duplicate)"
                );
            } else {
                ::tracing::trace!(
                    handler = ctx.handler_name,
                    key = %key,
                    "bloom: key added"
                );
            }
        }

        Some(Box::new(BloomFilterRequestGuard {}))
    }

    fn on_connect(
        &self,
        ctx: &afast::hook::RequestContext,
    ) -> Option<Box<dyn afast::hook::ConnectionGuard>> {
        // 长连接也写入 BloomFilter
        ctx.ctx.insert(self.filter.clone());

        if let Some(key) = self.extract_key(ctx) {
            let exists = {
                let bits = self.filter.bits.read().unwrap();
                let positions = self.filter.hash_positions(&key);
                positions.iter().all(|&pos| {
                    let index = pos / 64;
                    let offset = pos % 64;
                    index < bits.len() && (bits[index] & (1u64 << offset)) != 0
                })
            };

            ctx.ctx.insert(BloomFilterContext {
                key: key.clone(),
                exists,
            });

            {
                let mut bits = self.filter.bits.write().unwrap();
                let positions = self.filter.hash_positions(&key);
                for pos in positions {
                    let index = pos / 64;
                    let offset = pos % 64;
                    if index < bits.len() {
                        bits[index] |= 1u64 << offset;
                    }
                }
            }
        }

        Some(Box::new(BloomFilterConnectionGuard {}))
    }
}

// ═══════════════════════════════════════════════════════════════
//  Guards
// ═══════════════════════════════════════════════════════════════

/// 布隆过滤器请求 Guard
struct BloomFilterRequestGuard;

impl afast::hook::RequestGuard for BloomFilterRequestGuard {
    fn on_response(&mut self, _ctx: &afast::hook::RequestContext, _response: &[u8]) {
        #[cfg(feature = "log")]
        ::tracing::trace!("bloom: request completed");
    }

    fn on_error(&mut self, _ctx: &afast::hook::RequestContext, _error: &afast::Error) {
        #[cfg(feature = "log")]
        ::tracing::trace!("bloom: request errored");
    }
}

/// 布隆过滤器长连接 Guard
struct BloomFilterConnectionGuard;

impl afast::hook::ConnectionGuard for BloomFilterConnectionGuard {
    fn on_disconnect(&mut self, _ctx: &afast::hook::RequestContext) {
        #[cfg(feature = "log")]
        ::tracing::trace!("bloom: connection closed");
    }
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 布隆过滤器配置扩展
pub trait AFasterBloomExt {
    /// 链式配置 BloomFilter
    fn with_bloom(self, f: impl FnOnce(BloomFilter) -> BloomFilter) -> Self;
}

impl AFasterBloomExt for crate::AFaster {
    fn with_bloom(mut self, f: impl FnOnce(BloomFilter) -> BloomFilter) -> Self {
        self.state.bloom = f(self.state.bloom);
        self
    }
}
