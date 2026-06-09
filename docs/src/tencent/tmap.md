# 腾讯地图 (TMap)

> Feature: `tmap` | 依赖: `reqwest`, `serde_json`

> 📖 官方文档：<https://lbs.qq.com/service/webService/webServiceGuide>

## 简介

腾讯地图 Web 服务 API 封装，支持地理编码、逆地理编码、路径规划、POI 搜索、天气查询、IP 定位和坐标转换。

## 配置

```toml
[tmap]
key = "your_tencent_map_key"  # 腾讯地图 Web 服务 API Key (lbs.qq.com)
```

## API

所有方法返回 `Result<TmapResponse, afast::Error>`。

```rust
// 配置自动从 config.toml 的 [tmap] 段加载
let app = AFaster::new("config.toml".to_string()).await.unwrap();
let tmap = &app.state.tmap;  // 在 handler 中通过 state.tmap 访问
```

### TmapResponse

```rust
pub struct TmapResponse {
    pub status: i64,        // 0=成功
    pub message: String,    // "Success" 或错误描述
    pub request_id: String, // 请求唯一标识
    pub data: Value,        // 业务数据
}
```

- `response.is_ok()` — 快速判断是否成功

### 地址服务

```rust
// 地理编码: 地址 → 坐标
let res = tmap.geocoder("北京市海淀区彩和坊路海淀西大街74号", Some("北京")).await?;

// 逆地理编码: 坐标 → 地址
let res = tmap.reverse_geocoder("39.984154,116.307490", Some("1"), Some("address_format=short;radius=5000")).await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `address` | `&str` | 结构化地址 |
| `region` | `Option<&str>` | 指定城市，提高准确性 |
| `location` | `&str` | 坐标，格式 **纬度,经度**（注意顺序！） |
| `get_poi` | `Option<&str>` | 是否返回周边 POI |
| `poi_options` | `Option<&str>` | POI 控制参数 |

### 路径规划

```rust
// 驾车
let res = tmap.direction_driving("39.984154,116.307490", "39.904989,116.405285", None, Some("LEAST_TIME")).await?;

// 步行
let res = tmap.direction_walking("39.984154,116.307490", "39.904989,116.405285").await?;

// 骑行
let res = tmap.direction_bicycling("39.984154,116.307490", "39.904989,116.405285").await?;

// 电动车
let res = tmap.direction_ebicycling("39.984154,116.307490", "39.904989,116.405285").await?;

// 公交
let res = tmap.direction_transit("39.984154,116.307490", "39.904989,116.405285", Some("LEAST_TIME")).await?;

// 距离矩阵
let res = tmap.distance_matrix("driving", "39.984154,116.307490", "39.904989,116.405285;39.912345,116.387654").await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `from` | `&str` | 起点坐标，格式 **纬度,经度** |
| `to` | `&str` | 终点坐标，格式 **纬度,经度** |
| `waypoints` | `Option<&str>` | 途经点，格式 "lat1,lng1;lat2,lng2" |
| `policy` | `Option<&str>` | 策略: LEAST_TIME / LEAST_TRANSFER / LEAST_WALKING / LEAST_PRICE |
| `mode` | `&str` | 距离矩阵计算方式: driving / walking / bicycling |

### POI 搜索

```rust
// 周边搜索
let res = tmap.search("酒店", "nearby(39.984154,116.307490,1000)", Some("5"), Some("1")).await?;

// 城市搜索
let res = tmap.search("北京大学", "region(北京,0)", Some("5"), Some("1")).await?;

// 矩形搜索
let res = tmap.search("美食", "rectangle(39.907,116.368,39.914,116.379)", Some("5"), Some("1")).await?;

// 多边形搜索
let res = tmap.search_by_polygon("公园", "39.93,116.35;39.93,116.43;39.91,116.43;39.91,116.35", None, None).await?;

// 周边推荐（无需关键词）
let res = tmap.explore("nearby(39.984154,116.307490,1000)", Some("1")).await?;

// 关键词提示
let res = tmap.suggestion("北京大", Some("北京")).await?;

// POI 详情
let res = tmap.poi_detail("6621879543162709731").await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `keyword` | `&str` | 搜索关键字 |
| `boundary` | `&str` | 搜索范围格式 |
| `polygon` | `&str` | 多边形坐标 "lat1,lng1;lat2,lng2;..." |
| `page_size` | `Option<&str>` | 每页条数（最大 20） |
| `page_index` | `Option<&str>` | 页码 |

### 天气查询

```rust
// 实况天气（按 adcode）
let res = tmap.weather(Some("110000"), None, Some("now"), None).await?;

// 天气预报（含生活指数）
let res = tmap.weather(Some("110000"), None, Some("future"), Some("index")).await?;

// 逐小时预报（按坐标）
let res = tmap.weather(None, Some("39.984154,116.307490"), Some("hours"), None).await?;
```

### IP 定位

```rust
let res = tmap.ip_location(Some("114.242.249.146")).await?;
```

### 坐标转换

```rust
// GPS(WGS-84) → 腾讯(GCJ-02)
let res = tmap.coord_translate("39.984154,116.307490", "1").await?;

// 百度(BD-09) → 腾讯(GCJ-02)
let res = tmap.coord_translate("39.984154,116.307490", "3").await?;
```

## 错误码

| 错误码 | 说明 |
|--------|------|
| 41401 | 腾讯地图 Key 未配置 |
| 51401 | API 请求失败 (网络/HTTP 错误) |
| 51402 | API 响应 JSON 解析失败 |
| 51403 | API 返回业务错误 (status≠0) |

## 注意事项

- **坐标格式**: 腾讯地图使用 **纬度,经度** 顺序（与高德的经度,纬度相反！）
- **坐标系**: GCJ-02，GPS 原始坐标需先通过 `coord_translate` 转换
- **成功状态**: `status == 0`（不同于高德的 `status == "1"`）
- **路线时长**: 驾车 `duration` 单位为**分钟**，距离矩阵 `duration` 单位为**秒**
- 状态码详情: https://lbs.qq.com/service/webService/webServiceGuide/status

## 功能对比 (TMap vs AMap)

| 类别 | TMap | AMap |
|------|------|------|
| 地理编码 | ✅ | ✅ |
| 逆地理编码 | ✅ | ✅ (更多参数) |
| 驾车 | ✅ (waypoints支持) | ✅ (strategy扩展) |
| 步行 | ✅ | ✅ |
| 骑行 | ✅ | ✅ |
| 电动车 | ✅ | ❌ |
| 公交 | ✅ | ✅ |
| 距离测量 | ✅ (距离矩阵,更强大) | ✅ (单起点) |
| POI 关键词搜索 | ✅ (search) | ✅ (poi_text) |
| POI 周边搜索 | ⚠️ (search的nearby模式) | ✅ (poi_around,精细) |
| POI 多边形搜索 | ✅ | ✅ |
| POI 推荐 | ✅ (explore) | ❌ |
| POI 详情 | ✅ | ✅ |
| 关键词提示 | ✅ (suggestion) | ❌ |
| 天气 | ✅ (实况/预报/逐小时) | ✅ (实况/预报) |
| IP 定位 | ✅ | ❌ |
| 坐标转换 | ✅ | ❌ |
