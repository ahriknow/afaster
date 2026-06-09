# 高德地图 (AMap)

> Feature: `amap` | 依赖: `reqwest`, `serde_json`

> 📖 官方文档：<https://lbs.amap.com/api/webservice/summary>

## 简介

高德地图 Web 服务 API 封装，支持地理编码、逆地理编码、路径规划、POI 搜索、距离测量和天气查询。

## 配置

```toml
[amap]
key = "your_amap_key"  # 高德 Web 服务 API Key (console.amap.com)
```

## API

所有方法返回 `Result<AmapResponse, afast::Error>`。

```rust
// 配置自动从 config.toml 的 [amap] 段加载
let app = AFaster::new("config.toml".to_string()).await.unwrap();
let amap = &app.state.amap;  // 在 handler 中通过 state.amap 访问
```

### AmapResponse

```rust
pub struct AmapResponse {
    pub status: String,    // "1"=成功, "0"=失败
    pub info: String,      // "OK" 或错误描述
    pub infocode: String,  // "10000"=成功
    pub data: Value,       // 业务数据 (geocodes/regeocode/route/pois/lives/forecasts...)
}
```

- `response.is_ok()` — 快速判断是否成功

### 地理编码

```rust
// 地理编码: 地址 → 坐标
let res = amap.geocode("北京市朝阳区阜通东大街6号", Some("北京")).await?;

// 逆地理编码: 坐标 → 地址
let res = amap.regeocode("116.397428,39.90923", Some("base"), Some(1000)).await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `address` | `&str` | 结构化地址 |
| `city` | `Option<&str>` | 指定城市 |
| `location` | `&str` | 经度,纬度 |
| `extensions` | `Option<&str>` | `base`(基本信息) / `all`(详细信息) |
| `radius` | `Option<u32>` | 搜索半径 (米)，默认 1000 |

### 路径规划

```rust
// 步行
let res = amap.direction_walking("116.397428,39.90923", "116.407428,39.91923").await?;

// 驾车
let res = amap.direction_driving("116.397428,39.90923", "116.407428,39.91923", None, None).await?;

// 公交
let res = amap.direction_transit("116.397428,39.90923", "116.407428,39.91923", "北京", None, None).await?;

// 骑行 (v4 API)
let res = amap.direction_bicycling("116.397428,39.90923", "116.407428,39.91923").await?;

// 距离测量
let res = amap.distance("116.397428,39.90923", "116.407428,39.91923", None).await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `origin` | `&str` | 起点经度,纬度 |
| `destination` | `&str` | 终点经度,纬度 |
| `strategy` | `Option<u32>` | 驾车/公交策略编号 |
| `extensions` | `Option<&str>` | `base` / `all`(返回详细路段) |
| `city` | `&str` | 公交规划必填，终点城市 |
| `type_` | `Option<u32>` | 距离类型: 1=步行, 0=驾车 |

### POI 搜索

```rust
// 关键字搜索
let res = amap.poi_text("北京大学", Some("北京"), None, None, None, None).await?;

// 周边搜索
let res = amap.poi_around("116.397428,39.90923", Some("餐厅"), None, Some(1000), None, None, None, None).await?;

// 多边形搜索
let res = amap.poi_polygon("116.460988,40.006819|116.480988,40.006819|116.470988,39.996819", Some("肯德基"), None, None, None, None).await?;

// 详情搜索
let res = amap.poi_detail("B000A7BD6C").await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `keywords` | `&str` | 搜索关键字 |
| `city` | `Option<&str>` | 城市 |
| `citylimit` | `Option<bool>` | 是否限制在城市范围内 |
| `location` | `&str` | 中心点经度,纬度 |
| `types` | `Option<&str>` | POI 类型 |
| `radius` | `Option<u32>` | 搜索半径 (米) |
| `polygon` | `&str` | 多边形坐标，格式: `lng1,lat1|lng2,lat2|...` |
| `offset` | `Option<u32>` | 每页记录数 (默认 20) |
| `page` | `Option<u32>` | 页码 |
| `sortrule` | `Option<&str>` | 排序: `distance` / `weight` |
| `extensions` | `Option<&str>` | `base` / `all` |
| `id` | `&str` | POI ID |

### 天气查询

```rust
// 实时天气
let res = amap.weather("110000", None).await?;

// 预报天气
let res = amap.weather("110000", Some("all")).await?;
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `city` | `&str` | 城市编码 (adcode) |
| `extensions` | `Option<&str>` | `base`(实时) / `all`(预报) |

## 错误码

| 错误码 | 说明 |
|--------|------|
| 41301 | 高德 Key 未配置 |
| 51301 | API 请求失败 (网络/HTTP 错误) |
| 51302 | API 响应 JSON 解析失败 |
| 51303 | API 返回业务错误 (status≠"1") |

## 注意事项

- **坐标格式**: 高德地图使用 **经度,纬度** 顺序（与高德的经度,纬度相反！）
- 骑行路径使用 v4 API (`/v4/direction/bicycling`)，其余路径使用 v3
- 城市编码参考: [高德城市编码表](https://lbs.amap.com/demo/list/jsapi-v2/demo/citycode-list)
- 错误码详情: `response.info` + `response.infocode`

## 功能对比 (AMap vs TMap)

| 类别 | AMap | TMap |
|------|------|------|
| 地理编码 | ✅ | ✅ |
| 逆地理编码 | ✅ (更多参数) | ✅ |
| 驾车 | ✅ (strategy扩展) | ✅ (waypoints支持) |
| 步行 | ✅ | ✅ |
| 骑行 | ✅ | ✅ |
| 电动车 | ❌ | ✅ |
| 公交 | ✅ | ✅ |
| 距离测量 | ✅ (单起点) | ✅ (距离矩阵,更强大) |
| POI 关键词搜索 | ✅ (poi_text) | ✅ (search) |
| POI 周边搜索 | ✅ (poi_around,精细) | ⚠️ (search的nearby模式) |
| POI 多边形搜索 | ✅ | ✅ |
| POI 推荐 | ❌ | ✅ (explore) |
| POI 详情 | ✅ | ✅ |
| 关键词提示 | ❌ | ✅ (suggestion) |
| 天气 | ✅ (实况/预报) | ✅ (实况/预报/逐小时) |
| IP 定位 | ❌ | ✅ |
| 坐标转换 | ❌ | ✅ |
