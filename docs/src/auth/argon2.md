# Argon2 密码哈希

> Feature: `argon2-hash`

基于 [RustCrypto/argon2](https://docs.rs/argon2) 实现的密码哈希模块，支持 Argon2i、Argon2d、Argon2id 三种变体。

## 三种变体

| 变体 | 特点 | 适用场景 |
|------|------|---------|
| `Argon2i` | 抗侧信道攻击 | 密钥派生、硬件安全模块 |
| `Argon2d` | 抗 GPU/ASIC 破解 | 加密货币、纯数据保护 |
| `Argon2id` | 混合模式（**推荐**） | 用户密码哈希 |

> 📖 [OWASP 推荐](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html): 优先使用 Argon2id。

## 快速使用

```rust
use afaster::argon2::{self, Argon2Hasher, Argon2Variant, Argon2Config};

// ── 最简用法（默认 Argon2id） ──
let hash = argon2::hash("my_password")?;
let ok = argon2::verify("my_password", &hash)?;

// ── 自定义配置 ──
let hasher = Argon2Hasher::with_config(Argon2Config {
    variant: Argon2Variant::ID,
    memory_cost: 65536,  // 64 MB
    iterations: 3,
    parallelism: 4,
});
let hash = hasher.hash("my_password")?;
assert!(hasher.verify("my_password", &hash)?);

// ── 指定变体 ──
let hash_i = argon2::hash_with_variant("password", Argon2Variant::I)?;
let hash_d = argon2::hash_with_variant("password", Argon2Variant::D)?;

// ── 解析哈希参数 ──
let info = hasher.parse_hash(&hash)?;
println!("变体: {:?}, 内存: {}KB, 迭代: {}, 并行: {}",
    info.variant, info.memory_cost, info.iterations, info.parallelism);
```

## API

### Argon2Hasher

| 方法 | 说明 |
|------|------|
| `new()` | 默认配置（Argon2id, 19MB, 2 iterations, 1 parallelism） |
| `with_config(config)` | 自定义配置 |
| `hash(password)` | 哈希密码（自动生成随机 salt） |
| `hash_with_salt(password, salt)` | 使用指定 salt 哈希 |
| `verify(password, hash)` | 验证密码 |
| `parse_hash(hash)` | 解析哈希参数信息 |

### Argon2Config

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `variant` | `Argon2Variant` | `ID` | 变体类型 |
| `memory_cost` | `u32` | `19456` | 内存成本 (KB) |
| `iterations` | `u32` | `2` | 迭代次数 |
| `parallelism` | `u32` | `1` | 并行度 |

### 便捷函数

| 函数 | 说明 |
|------|------|
| `argon2::hash(password)` | 默认配置哈希 |
| `argon2::verify(password, hash)` | 默认配置验证 |
| `argon2::hash_with_variant(password, variant)` | 指定变体哈希 |

## OWASP 推荐参数

| 变体 | 内存 | 迭代 | 并行 |
|------|------|------|------|
| Argon2id | ≥ 19 MB (19456 KB) | 2 | 1 |
| Argon2i | ≥ 19 MB (19456 KB) | 2 | 1 |
| Argon2d | ≥ 19 MB (19456 KB) | 2 | 1 |

> 高安全场景可提升至 64 MB + 3 iterations + 4 parallelism。

## 哈希格式

输出为 PHC 格式：`$argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>`
