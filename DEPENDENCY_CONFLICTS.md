# 依赖版本冲突记录

## sha2-v10 (digest 0.10 vs 0.11)

**问题**: `rsa 0.9` 依赖 `digest 0.10`，而 `sha2 0.11` 依赖 `digest 0.11`，两个版本的 `Digest` trait 不兼容。

**现状**:
- `sha2 0.11.0` → `digest 0.11.3` (nonce/hmac 等模块使用)
- `rsa 0.9.10` → `digest 0.10.7` (微信支付 V3 签名使用)
- 微信支付签名需要 `sha2 0.10`，因此 Cargo.toml 中有：
  ```toml
  sha2-v10 = { package = "sha2", version = "0.10", features = ["oid"], optional = true }
  ```

**解决方案**: 等 `rsa` crate 升级到支持 `digest 0.11` 的版本后：
1. 删除 Cargo.toml 中的 `sha2-v10` 依赖
2. 将 `wx-pay-*` features 中的 `dep:sha2-v10` 改为 `sha2`
3. 更新 `src/state/wx_pay/mod.rs` 中的 `use sha2_v10::Sha256` 为 `use sha2::Sha256`

**检查方式**:
```bash
cargo tree --features wx-pay-h5 -e normal | grep -E "sha2|digest|rsa"
```
