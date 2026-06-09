use afaster::redis::{Redis, RedisConfig};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    redis: RedisConfig,
}

struct Stats {
    ok: u32,
    fail: u32,
}

impl Stats {
    fn new() -> Self {
        Stats { ok: 0, fail: 0 }
    }

    fn record_ok(&mut self, label: &str, detail: &str) {
        self.ok += 1;
        println!("  ✅ [{}] {}", label, detail);
    }

    fn record_err(&mut self, label: &str, err: &impl std::fmt::Display) {
        self.fail += 1;
        println!("  ❌ [{}] 失败: {}", label, err);
    }

    fn summary(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!(
            "║  测试完成: ✅ {} 成功  ❌ {} 失败  共 {} 项",
            self.ok,
            self.fail,
            self.ok + self.fail
        );
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              Redis 全功能测试                            ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // ── 初始化 ────────────────────────────────────────────────
    let config: Config = toml::from_str(
        &std::fs::read_to_string("examples/redis/config.toml").expect("读取 config.toml 失败"),
    )
    .expect("解析 config.toml 失败");

    let redis = match Redis::connect(&config.redis).await {
        Ok(r) => {
            println!(
                "  ✅ 连接成功: {}:{}\n",
                config.redis.host, config.redis.port
            );
            r
        }
        Err(e) => {
            println!("  ❌ 连接失败: {}", e);
            return;
        }
    };

    let mut stats = Stats::new();

    // ════════════════════════════════════════════════════════════
    //  1. 基础操作: SET / GET / DEL
    // ════════════════════════════════════════════════════════════
    println!("▶ [1/11] 基础操作: SET / GET / DEL");

    match redis.set("example:name", "afaster", None).await {
        Ok(_) => {
            let val: Option<String> = redis.get("example:name").await.unwrap();
            stats.record_ok("SET+GET", &format!("name = {:?}", val));
            redis.del("example:name").await.ok();
        }
        Err(e) => stats.record_err("SET+GET", &e),
    }

    // ════════════════════════════════════════════════════════════
    //  2. SET 带过期时间
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [2/11] SET 带过期时间 (TTL)");

    match redis.set("example:temp", "will_expire", Some(60)).await {
        Ok(_) => {
            let ttl = redis.ttl("example:temp").await.unwrap();
            stats.record_ok("SET EX", &format!("TTL = {}s", ttl));
            redis.del("example:temp").await.ok();
        }
        Err(e) => stats.record_err("SET EX", &e),
    }

    // ════════════════════════════════════════════════════════════
    //  3. SET NX (仅当不存在时设置)
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [3/11] SET NX (幂等设置)");

    redis.del("example:nx").await.ok();
    match redis.set_nx("example:nx", "first", None).await {
        Ok(ok1) => {
            let ok2 = redis.set_nx("example:nx", "second", None).await.unwrap();
            let val: Option<String> = redis.get("example:nx").await.unwrap();
            stats.record_ok("SET NX", &format!("首次={} 再次={} 值={:?}", ok1, ok2, val));
            redis.del("example:nx").await.ok();
        }
        Err(e) => stats.record_err("SET NX", &e),
    }

    // ════════════════════════════════════════════════════════════
    //  4. EXISTS / EXPIRE / TTL
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [4/11] EXISTS / EXPIRE / TTL");

    redis.set("example:ttl", "val", None).await.unwrap();
    let exists = redis.exists("example:ttl").await.unwrap();
    let ttl_before = redis.ttl("example:ttl").await.unwrap();
    redis.expire("example:ttl", 120).await.unwrap();
    let ttl_after = redis.ttl("example:ttl").await.unwrap();
    stats.record_ok(
        "EXISTS/EXPIRE/TTL",
        &format!(
            "exists={} ttl_before={} ttl_after={}",
            exists, ttl_before, ttl_after
        ),
    );
    redis.del("example:ttl").await.ok();

    // ════════════════════════════════════════════════════════════
    //  5. INCR / DECR
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [5/11] INCR 原子自增/自减");

    redis.del("example:counter").await.ok();
    let v1 = redis.incr("example:counter", 1).await.unwrap();
    let v2 = redis.incr("example:counter", 10).await.unwrap();
    let v3 = redis.incr("example:counter", -3).await.unwrap();
    stats.record_ok("INCR", &format!("+1={} +10={} -3={}", v1, v2, v3));
    redis.del("example:counter").await.ok();

    // ════════════════════════════════════════════════════════════
    //  6. KEYS
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [6/11] KEYS 模式匹配");

    redis.set("example:k1", "a", None).await.unwrap();
    redis.set("example:k2", "b", None).await.unwrap();
    redis.set("example:k3", "c", None).await.unwrap();
    let keys = redis.keys("example:k*").await.unwrap();
    stats.record_ok("KEYS", &format!("example:k* => {:?}", keys));
    redis.del("example:k1").await.ok();
    redis.del("example:k2").await.ok();
    redis.del("example:k3").await.ok();

    // ════════════════════════════════════════════════════════════
    //  7. Hash: HSET / HGET / HGETALL / HINCRBY / HDEL
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [7/11] Hash 操作");

    redis.del("example:user").await.ok();
    redis.hset("example:user", "name", "Alice").await.unwrap();
    redis.hset("example:user", "age", "25").await.unwrap();
    redis.hset("example:user", "city", "Beijing").await.unwrap();

    let name: Option<String> = redis.hget("example:user", "name").await.unwrap();
    let all: std::collections::HashMap<String, String> =
        redis.hgetall("example:user").await.unwrap();
    let new_age = redis.hincrby("example:user", "age", 1).await.unwrap();
    redis.hdel("example:user", "city").await.unwrap();
    let remaining: std::collections::HashMap<String, String> =
        redis.hgetall("example:user").await.unwrap();

    stats.record_ok(
        "Hash",
        &format!(
            "HGET name={:?} HGETALL={:?} HINCRBY age+1={} HDEL后={:?}",
            name, all, new_age, remaining
        ),
    );
    redis.del("example:user").await.ok();

    // ════════════════════════════════════════════════════════════
    //  8. 分布式锁
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [8/11] 分布式锁 (lock / unlock)");

    let lock1 = redis
        .lock("example:resource", "uuid-aaa", 10)
        .await
        .unwrap();
    let lock2 = redis
        .lock("example:resource", "uuid-bbb", 10)
        .await
        .unwrap();
    let unlock_ok = redis.unlock("example:resource", "uuid-aaa").await.unwrap();
    let unlock_wrong = redis.unlock("example:resource", "uuid-bbb").await.unwrap();

    stats.record_ok(
        "Lock",
        &format!(
            "lock1={} lock2(冲突)={} unlock={} unlock_wrong_value={}",
            lock1, lock2, unlock_ok, unlock_wrong
        ),
    );

    // ════════════════════════════════════════════════════════════
    //  9. PUBLISH
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [9/11] PUBLISH 发布消息");

    match redis.publish("example:channel", "hello afaster").await {
        Ok(count) => {
            stats.record_ok("PUBLISH", &format!("订阅者数量 = {}", count));
        }
        Err(e) => stats.record_err("PUBLISH", &e),
    }

    // ════════════════════════════════════════════════════════════
    //  10. Key 前缀
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [10/11] Key 前缀");

    let prefix_config = RedisConfig {
        host: config.redis.host.clone(),
        port: config.redis.port,
        db: 0,
        password: String::new(),
        prefix: "myprefix".to_string(),
    };
    match Redis::connect(&prefix_config).await {
        Ok(prefix_redis) => {
            prefix_redis
                .set("pfx_test", "prefixed", None)
                .await
                .unwrap();
            let val: Option<String> = prefix_redis.get("pfx_test").await.unwrap();
            // 直接查原始 key 验证前缀
            let raw: Option<String> = redis.get("myprefix:pfx_test").await.unwrap();
            stats.record_ok(
                "Prefix",
                &format!(
                    "get(\"pfx_test\")={:?}  raw_get(\"myprefix:pfx_test\")={:?}",
                    val, raw
                ),
            );
            redis.del("myprefix:pfx_test").await.ok();
        }
        Err(e) => stats.record_err("Prefix", &e),
    }

    // ════════════════════════════════════════════════════════════
    //  11. 整数存取
    // ════════════════════════════════════════════════════════════
    println!("\n▶ [11/11] 整数存取");

    redis.set("example:num", 42_i64, None).await.unwrap();
    let num: Option<i64> = redis.get("example:num").await.unwrap();
    stats.record_ok("i64", &format!("set(42) get={:?}", num));
    redis.del("example:num").await.ok();

    // ── 清理 & 汇总 ───────────────────────────────────────────
    println!("\n──────────────────────────────────────────────────────────");
    stats.summary();
}
