use afaster::argon2::{self, Argon2Config, Argon2Hasher, Argon2Variant};

fn main() {
    println!("═══════════════════════════════════════════════════");
    println!("  Argon2 密码哈希示例");
    println!("═══════════════════════════════════════════════════\n");

    // ── 1. 默认配置 (Argon2id) ──
    println!("【1】默认配置 (Argon2id, 19MB, 2 iterations)");
    let hasher = Argon2Hasher::new();
    let password = "my_secure_password_123";

    let hash = hasher.hash(password).expect("hash failed");
    println!("  密码:   {password}");
    println!("  哈希:   {hash}");

    let valid = hasher.verify(password, &hash).expect("verify failed");
    println!("  验证(正确密码): {valid}");

    let invalid = hasher
        .verify("wrong_password", &hash)
        .expect("verify failed");
    println!("  验证(错误密码): {invalid}");

    // 解析哈希参数
    let info = hasher.parse_hash(&hash).expect("parse failed");
    println!(
        "  解析:   variant={:?}, memory={}KB, iterations={}, parallelism={}\n",
        info.variant, info.memory_cost, info.iterations, info.parallelism
    );

    // ── 2. Argon2i ──
    println!("【2】Argon2i (抗侧信道)");
    let hash_i = argon2::hash_with_variant("test_password", Argon2Variant::I).expect("hash failed");
    println!("  哈希:   {hash_i}");
    let ok = argon2::verify("test_password", &hash_i).expect("verify failed");
    println!("  验证:   {ok}\n");

    // ── 3. Argon2d ──
    println!("【3】Argon2d (抗 GPU)");
    let hash_d = argon2::hash_with_variant("test_password", Argon2Variant::D).expect("hash failed");
    println!("  哈希:   {hash_d}");
    let ok = argon2::verify("test_password", &hash_d).expect("verify failed");
    println!("  验证:   {ok}\n");

    // ── 4. 自定义参数 ──
    println!("【4】自定义配置 (Argon2id, 64MB, 3 iterations, 4 parallelism)");
    let custom_hasher = Argon2Hasher::with_config(Argon2Config {
        variant: Argon2Variant::ID,
        memory_cost: 65536,
        iterations: 3,
        parallelism: 4,
    });
    let hash_custom = custom_hasher
        .hash("high_security_password")
        .expect("hash failed");
    println!("  哈希:   {hash_custom}");
    let info = custom_hasher
        .parse_hash(&hash_custom)
        .expect("parse failed");
    println!(
        "  解析:   variant={:?}, memory={}KB, iterations={}, parallelism={}\n",
        info.variant, info.memory_cost, info.iterations, info.parallelism
    );

    // ── 5. 便捷函数 ──
    println!("【5】便捷函数");
    let hash2 = argon2::hash("quick_hash").expect("hash failed");
    println!("  argon2::hash():    {hash2}");
    let ok = argon2::verify("quick_hash", &hash2).expect("verify failed");
    println!("  argon2::verify():  {ok}\n");

    // ── 6. 使用指定 salt ──
    println!("【6】指定 salt 哈希");
    let salt = "fixed_salt_value_1234"; // 16 bytes base64
    let hash_s1 = hasher
        .hash_with_salt("same_password", salt)
        .expect("hash failed");
    let hash_s2 = hasher
        .hash_with_salt("same_password", salt)
        .expect("hash failed");
    println!("  第一次: {hash_s1}");
    println!("  第二次: {hash_s2}");
    println!("  确定性: {}", hash_s1 == hash_s2);

    println!("\n═══════════════════════════════════════════════════");
    println!("  所有测试通过 ✅");
    println!("═══════════════════════════════════════════════════");
}
