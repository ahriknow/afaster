use afaster::tmap::{Tmap, TmapResponse};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    tmap: Tmap,
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

    fn record(&mut self, label: &str, result: &Result<TmapResponse, impl std::fmt::Display>) {
        match result {
            Ok(resp) => {
                self.ok += 1;
                println!(
                    "\n  ✅ [{}] 成功  status={} message={}",
                    label, resp.status, resp.message
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
    println!("║           腾讯地图 API 全接口测试                       ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    // ── 初始化 ────────────────────────────────────────────────
    let config: Config = toml::from_str(
        &std::fs::read_to_string("examples/tmap/config.toml").expect("读取 config.toml 失败"),
    )
    .expect("解析 config.toml 失败");
    let mut tmap = config.tmap;
    tmap.client = reqwest::Client::new();

    let mut stats = Stats::new();

    // ════════════════════════════════════════════════════════════
    //  1. 地址解析（地理编码）
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [1/11] 地址解析 (geocoder)");
    let r = tmap
        .geocoder("北京市海淀区彩和坊路海淀西大街74号", Some("北京"))
        .await;
    stats.record("地址解析", &r);

    // ════════════════════════════════════════════════════════════
    //  2. 逆地址解析（逆地理编码）
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [2/11] 逆地址解析 (reverse_geocoder)");
    let r = tmap
        .reverse_geocoder(
            "39.984154,116.307490",
            Some("1"),
            Some("address_format=short;radius=5000"),
        )
        .await;
    stats.record("逆地址解析", &r);

    // ════════════════════════════════════════════════════════════
    //  3. 驾车路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [3/11] 驾车路径规划 (direction_driving)");
    let r = tmap
        .direction_driving(
            "39.984154,116.307490",
            "39.904989,116.405285",
            None,
            Some("LEAST_TIME"),
        )
        .await;
    stats.record("驾车路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  4. 步行路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [4/11] 步行路径规划 (direction_walking)");
    let r = tmap
        .direction_walking("39.984154,116.307490", "39.904989,116.405285")
        .await;
    stats.record("步行路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  5. 骑行路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [5/11] 骑行路径规划 (direction_bicycling)");
    let r = tmap
        .direction_bicycling("39.984154,116.307490", "39.904989,116.405285")
        .await;
    stats.record("骑行路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  6. 电动车路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [6/11] 电动车路径规划 (direction_ebicycling)");
    let r = tmap
        .direction_ebicycling("39.984154,116.307490", "39.904989,116.405285")
        .await;
    stats.record("电动车路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  7. 公交路径规划
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [7/11] 公交路径规划 (direction_transit)");
    let r = tmap
        .direction_transit(
            "39.984154,116.307490",
            "39.904989,116.405285",
            Some("LEAST_TIME"),
        )
        .await;
    stats.record("公交路径规划", &r);

    // ════════════════════════════════════════════════════════════
    //  8. 距离矩阵
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [8/11] 距离矩阵 (distance_matrix)");
    let r = tmap
        .distance_matrix(
            "driving",
            "39.984154,116.307490",
            "39.904989,116.405285;39.912345,116.387654",
        )
        .await;
    stats.record("距离矩阵", &r);

    // ════════════════════════════════════════════════════════════
    //  9. POI 搜索
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [9a/11] 地点搜索 - 周边 (search nearby)");
    let r = tmap
        .search(
            "酒店",
            "nearby(39.984154,116.307490,1000)",
            Some("5"),
            Some("1"),
        )
        .await;
    stats.record("周边搜索", &r);

    println!("\n\n▶ [9b/11] 地点搜索 - 城市 (search region)");
    let r = tmap
        .search("北京大学", "region(北京,0)", Some("5"), Some("1"))
        .await;
    stats.record("城市搜索", &r);

    println!("\n\n▶ [9c/11] 地点搜索 - 矩形 (search rectangle)");
    let r = tmap
        .search(
            "美食",
            "rectangle(39.907293,116.368935,39.914996,116.379321)",
            Some("5"),
            Some("1"),
        )
        .await;
    stats.record("矩形搜索", &r);

    // ════════════════════════════════════════════════════════════
    //  10. 天气查询
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [10a/11] 天气查询 - 实况 (weather now)");
    let r = tmap.weather(Some("110000"), None, Some("now"), None).await;
    stats.record("实况天气", &r);

    println!("\n\n▶ [10b/11] 天气查询 - 预报 (weather future)");
    let r = tmap
        .weather(Some("110000"), None, Some("future"), Some("index"))
        .await;
    stats.record("天气预报", &r);

    // ════════════════════════════════════════════════════════════
    //  11. IP 定位
    // ════════════════════════════════════════════════════════════
    println!("\n\n▶ [11/11] IP 定位 (ip_location)");
    let r = tmap.ip_location(Some("114.242.249.146")).await;
    stats.record("IP 定位", &r);

    // ── 汇总 ──────────────────────────────────────────────────
    stats.summary();
}
