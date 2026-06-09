use afaster::amap::{Amap, AmapResponse};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    amap: Amap,
}

/// 测试结果计数
struct Stats {
    ok: u32,
    fail: u32,
}

impl Stats {
    fn new() -> Self {
        Stats { ok: 0, fail: 0 }
    }

    fn record(&mut self, label: &str, result: &Result<AmapResponse, impl std::fmt::Display>) {
        match result {
            Ok(resp) => {
                self.ok += 1;
                println!(
                    "\n  ✅ [{}] 成功  status={} info={}",
                    label, resp.status, resp.info
                );
                let json = serde_json::to_string_pretty(&resp.data).unwrap_or_default();
                let preview: String = json.chars().take(1500).collect();
                println!("  {}", preview.lines().collect::<Vec<_>>().join("\n  "));
                if json.len() > 1500 {
                    println!("  ... ({} 字符，已截断)", json.len());
                }
            }
            Err(e) => {
                self.fail += 1;
                println!("\n  ❌ [{}] 失败: {}", label, e);
            }
        }
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
    println!("║           高德地图 API 全接口测试                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    // ── 初始化 ────────────────────────────────────────────────
    let config: Config = toml::from_str(
        &std::fs::read_to_string("examples/amap/config.toml").expect("读取 config.toml 失败"),
    )
    .expect("解析 config.toml 失败");
    let mut amap = config.amap;
    amap.client = reqwest::Client::new();

    let mut stats = Stats::new();

    // ════════════════════════════════════════════════════════════
    //  1. 地理编码：地址 → 坐标
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [1/12] 地理编码 (geocode)");
    let r = amap
        .geocode("北京市朝阳区阜通东大街6号", Some("北京"))
        .await;
    stats.record("地理编码", &r);

    // ════════════════════════════════════════════════════════════
    //  2. 逆地理编码：坐标 → 地址
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [2/12] 逆地理编码 (regeocode)");
    let r = amap
        .regeocode("116.397428,39.90923", Some("base"), Some("1000"))
        .await;
    stats.record("逆地理编码", &r);

    // ════════════════════════════════════════════════════════════
    //  3. 步行路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [3/12] 步行路径规划 (direction_walking)");
    let r = amap
        .direction_walking("116.397428,39.90923", "116.407428,39.91923")
        .await;
    stats.record("步行路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  4. 驾车路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [4/12] 驾车路径规划 (direction_driving)");
    let r = amap
        .direction_driving(
            "116.397428,39.90923",
            "116.407428,39.91923",
            Some("0"),
            Some("base"),
        )
        .await;
    stats.record("驾车路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  5. 公交路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [5/12] 公交路径规划 (direction_transit)");
    let r = amap
        .direction_transit(
            "116.397428,39.90923",
            "116.407428,39.91923",
            "北京",
            Some("0"),
            Some("base"),
        )
        .await;
    stats.record("公交路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  6. 骑行路径规划 (v4 API)
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [6/12] 骑行路径规划 (direction_bicycling)");
    let r = amap
        .direction_bicycling("116.397428,39.90923", "116.407428,39.91923")
        .await;
    stats.record("骑行路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  7. 距离测量
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [7/12] 距离测量 (distance)");
    let r = amap
        .distance("116.397428,39.90923", "116.407428,39.91923", Some("1"))
        .await;
    stats.record("距离测量", &r);

    // ════════════════════════════════════════════════════════════
    //  8. POI 关键字搜索
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [8/12] POI 关键字搜索 (poi_text)");
    let r = amap
        .poi_text(
            "北京大学",
            Some("北京"),
            Some("true"),
            Some("5"),
            Some("1"),
            Some("base"),
        )
        .await;
    stats.record("POI 关键字搜索", &r);

    // ════════════════════════════════════════════════════════════
    //  9. POI 周边搜索
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [9/12] POI 周边搜索 (poi_around)");
    let r = amap
        .poi_around(
            "116.397428,39.90923",
            Some("餐厅"),
            None,
            Some("1000"),
            Some("5"),
            Some("1"),
            Some("distance"),
            Some("base"),
        )
        .await;
    stats.record("POI 周边搜索", &r);

    // ════════════════════════════════════════════════════════════
    //  10. POI 多边形搜索
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [10/12] POI 多边形搜索 (poi_polygon)");
    let r = amap.poi_polygon(
        "116.460988,40.006819|116.480988,40.006819|116.480988,39.986819|116.460988,39.986819|116.460988,40.006819",
        Some("肯德基"),
        None,
        Some("5"),
        Some("1"),
        Some("base"),
    ).await;
    stats.record("POI 多边形搜索", &r);

    // ════════════════════════════════════════════════════════════
    //  11. POI 详情 (需要先从搜索结果获取一个 POI ID)
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [11/12] POI 详情 (poi_detail)");
    // 先从关键字搜索结果中提取一个 POI ID
    let search_result = amap
        .poi_text(
            "天安门",
            Some("北京"),
            None,
            Some("1"),
            Some("1"),
            Some("base"),
        )
        .await;
    let poi_id = match &search_result {
        Ok(resp) => resp
            .data
            .get("pois")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|poi| poi.get("id"))
            .and_then(|id| id.as_str())
            .map(|s| s.to_string()),
        Err(_) => None,
    };

    match poi_id {
        Some(ref id) => {
            println!("  使用 POI ID: {}", id);
            let r = amap.poi_detail(id).await;
            stats.record("POI 详情", &r);
        }
        None => {
            println!("  ⚠️  无法从搜索结果获取 POI ID，跳过详情测试");
            stats.fail += 1;
        }
    }

    // ════════════════════════════════════════════════════════════
    //  12. 天气查询（实况 + 预报）
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [12a/12] 天气查询 - 实况 (weather base)");
    let r = amap.weather("110000", Some("base")).await;
    stats.record("天气查询-实况", &r);

    println!("\n\n▶ [12b/12] 天气查询 - 预报 (weather all)");
    let r = amap.weather("110000", Some("all")).await;
    stats.record("天气查询-预报", &r);

    // ── 汇总 ──────────────────────────────────────────────────
    stats.summary();
}
