use std::collections::hash_map::DefaultHasher;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// 随机字符串生成器
///
/// 基于系统熵（`RandomState`）+ 原子计数器，每次调用生成唯一且不可预测的字符串。
/// 适用于 OAuth2 state、CSRF token 等场景。
#[derive(Clone)]
pub struct Nonce {
    seed: u64,
    counter: std::sync::Arc<AtomicU64>,
}

impl Nonce {
    pub fn new() -> Self {
        // RandomState 内部使用 OS 熵源（Linux: getrandom, Windows: BCryptGenRandom）
        // 用它生成一个高质量的随机 seed
        let seed = {
            let rb = std::collections::hash_map::RandomState::new();
            let mut h = rb.build_hasher();
            h.write_u64(0xDEAD_BEEF_CAFE_BABE);
            h.finish()
        };
        Nonce {
            seed,
            counter: std::sync::Arc::new(AtomicU64::new(0)),
        }
    }

    /// 生成 16 字符的十六进制随机字符串
    pub fn generate(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        let mut h = DefaultHasher::new();
        self.seed.hash(&mut h);
        n.hash(&mut h);
        // 混入系统时间纳秒，增加不可预测性
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
            .hash(&mut h);
        format!("{:016x}", h.finish())
    }

    /// 生成指定长度的十六进制随机字符串，最少 16 字符
    pub fn generate_len(&self, len: usize) -> String {
        let mut result = String::with_capacity(len);
        while result.len() < len {
            result.push_str(&self.generate());
        }
        result.truncate(len);
        result
    }
}
