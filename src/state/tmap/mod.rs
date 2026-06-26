pub(crate) mod err;
use err::*;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════
//  腾讯地图 Web 服务
// ═══════════════════════════════════════════════════════════════

const TMAP_BASE: &str = "https://apis.map.qq.com";

/// 腾讯地图 Web 服务 API 模块
///
/// 支持：地理/逆地理编码、路径规划、POI 搜索、天气查询、IP 定位
///
/// API: https://lbs.qq.com/service/webService/webServiceGuide
#[derive(Clone, Deserialize)]
pub struct Tmap {
    /// Web 服务 API Key
    pub key: String,
    #[serde(skip)]
    pub client: Client,
}

// ── 通用响应 ──────────────────────────────────────────────────

/// 腾讯地图通用响应结构
///
/// 所有 API 共享相同的顶层字段：status / message / request_id。
/// 业务数据通过 `data` 获取（`#[serde(flatten)]` 自动收集剩余字段）。
#[derive(Debug, Deserialize)]
pub struct TmapResponse {
    /// 状态码，0 为成功
    #[serde(default)]
    pub status: i64,
    /// 状态说明
    #[serde(default)]
    pub message: String,
    /// 请求唯一标识
    #[serde(default)]
    pub request_id: String,
    /// 业务数据
    #[serde(flatten)]
    pub data: Value,
}

impl TmapResponse {
    /// 请求是否成功
    #[inline]
    pub fn is_ok(&self) -> bool {
        self.status == 0
    }

    /// 获取错误信息
    pub fn error_info(&self) -> String {
        format!("{} ({})", self.message, self.status)
    }
}

// ── 实现 ──────────────────────────────────────────────────────

impl Default for Tmap {
    fn default() -> Self {
        Self::new()
    }
}

impl Tmap {
    pub fn new() -> Self {
        Tmap {
            key: String::new(),
            client: Client::new(),
        }
    }

    // ── 内部通用请求 ──────────────────────────────────────────

    /// 发送 GET 请求到腾讯地图 API
    async fn get(&self, path: &str, params: &[(&str, &str)]) -> crate::Result<TmapResponse> {
        if self.key.is_empty() {
            return Err(missing_key());
        }

        let url = format!("{}{}", TMAP_BASE, path);

        let mut query: Vec<(&str, &str)> = vec![("key", &self.key[..])];
        query.extend_from_slice(params);
        query.push(("output", "JSON"));

        let resp = self
            .client
            .get(&url)
            .query(&query)
            .send()
            .await
            .map_err(|e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 51401, msg = "TMap request failed" }, "{}", e);
                request_failed(&e.to_string())
            })?;

        let text = resp.text().await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51402, msg = "TMap response read failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        let tmap_resp: TmapResponse = serde_json::from_str(&text).map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51402, msg = "TMap response parse failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        if !tmap_resp.is_ok() {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 51403, msg = "TMap API error" },
                "{}", tmap_resp.error_info()
            );
            return Err(api_error(&tmap_resp.message, tmap_resp.status));
        }

        Ok(tmap_resp)
    }

    // ═══════════════════════════════════════════════════════════
    //  地址服务（地理编码 / 逆地理编码）
    // ═══════════════════════════════════════════════════════════

    /// 地址解析（地理编码）：地址 → 坐标
    ///
    /// - `address` — 结构化地址，建议包含城市名
    /// - `region` — 指定城市（可选），提高准确性
    ///
    /// API: `GET /ws/geocoder/v1/`
    pub async fn geocoder(
        &self,
        address: &str,
        region: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("address", address)];
        if let Some(r) = region {
            params.push(("region", r));
        }
        self.get("/ws/geocoder/v1/", &params).await
    }

    /// 逆地址解析（逆地理编码）：坐标 → 地址
    ///
    /// - `location` — 坐标，格式 "纬度,经度"（注意：纬度在前！）
    /// - `get_poi` — 是否返回周边 POI
    /// - `poi_options` — POI 控制参数
    ///
    /// API: `GET /ws/geocoder/v1/`
    pub async fn reverse_geocoder(
        &self,
        location: &str,
        get_poi: Option<&str>,
        poi_options: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("location", location)];
        if let Some(g) = get_poi {
            params.push(("get_poi", g));
        }
        if let Some(p) = poi_options {
            params.push(("poi_options", p));
        }
        self.get("/ws/geocoder/v1/", &params).await
    }

    // ═══════════════════════════════════════════════════════════
    //  路径规划
    // ═══════════════════════════════════════════════════════════

    /// 驾车路径规划
    ///
    /// - `from` — 起点坐标，格式 "纬度,经度"
    /// - `to` — 终点坐标，格式 "纬度,经度"
    /// - `waypoints` — 途经点（可选），格式 "lat1,lng1;lat2,lng2"
    /// - `policy` — 路线策略（可选），如 "LEAST_TIME,REAL_TRAFFIC"
    ///
    /// API: `GET /ws/direction/v1/driving/`
    pub async fn direction_driving(
        &self,
        from: &str,
        to: &str,
        waypoints: Option<&str>,
        policy: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("from", from), ("to", to)];
        if let Some(w) = waypoints {
            params.push(("waypoints", w));
        }
        if let Some(p) = policy {
            params.push(("policy", p));
        }
        self.get("/ws/direction/v1/driving/", &params).await
    }

    /// 步行路径规划
    ///
    /// - `from` — 起点坐标，格式 "纬度,经度"
    /// - `to` — 终点坐标，格式 "纬度,经度"
    ///
    /// API: `GET /ws/direction/v1/walking/`
    pub async fn direction_walking(&self, from: &str, to: &str) -> crate::Result<TmapResponse> {
        self.get("/ws/direction/v1/walking/", &[("from", from), ("to", to)])
            .await
    }

    /// 骑行路径规划
    ///
    /// - `from` — 起点坐标，格式 "纬度,经度"
    /// - `to` — 终点坐标，格式 "纬度,经度"
    ///
    /// API: `GET /ws/direction/v1/bicycling/`
    pub async fn direction_bicycling(&self, from: &str, to: &str) -> crate::Result<TmapResponse> {
        self.get("/ws/direction/v1/bicycling/", &[("from", from), ("to", to)])
            .await
    }

    /// 电动车路径规划
    ///
    /// - `from` — 起点坐标，格式 "纬度,经度"
    /// - `to` — 终点坐标，格式 "纬度,经度"
    ///
    /// API: `GET /ws/direction/v1/ebicycling/`
    pub async fn direction_ebicycling(&self, from: &str, to: &str) -> crate::Result<TmapResponse> {
        self.get(
            "/ws/direction/v1/ebicycling/",
            &[("from", from), ("to", to)],
        )
        .await
    }

    /// 公交路径规划
    ///
    /// - `from` — 起点坐标，格式 "纬度,经度"
    /// - `to` — 终点坐标，格式 "纬度,经度"
    /// - `policy` — 路线策略（可选）：LEAST_TIME / LEAST_TRANSFER / LEAST_WALKING / LEAST_PRICE
    ///
    /// API: `GET /ws/direction/v1/transit/`
    pub async fn direction_transit(
        &self,
        from: &str,
        to: &str,
        policy: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("from", from), ("to", to)];
        if let Some(p) = policy {
            params.push(("policy", p));
        }
        self.get("/ws/direction/v1/transit/", &params).await
    }

    /// 距离矩阵（批量距离计算）
    ///
    /// - `mode` — 计算方式：driving / walking / bicycling
    /// - `from` — 起点坐标串，格式 "lat1,lng1;lat2,lng2"
    /// - `to` — 终点坐标串，格式 "lat1,lng1;lat2,lng2"
    ///
    /// API: `GET /ws/distance/v1/matrix`
    pub async fn distance_matrix(
        &self,
        mode: &str,
        from: &str,
        to: &str,
    ) -> crate::Result<TmapResponse> {
        self.get(
            "/ws/distance/v1/matrix",
            &[("mode", mode), ("from", from), ("to", to)],
        )
        .await
    }

    // ═══════════════════════════════════════════════════════════
    //  POI 搜索
    // ═══════════════════════════════════════════════════════════

    /// 地点搜索（周边 / 城市 / 矩形）
    ///
    /// - `keyword` — 搜索关键字
    /// - `boundary` — 搜索范围：
    ///   - 周边: "nearby(lat,lng,radius)"
    ///   - 城市: "region(city_name)"
    ///   - 矩形: "rectangle(lat1,lng1,lat2,lng2)"
    /// - `page_size` — 每页条数（可选，最大 20）
    /// - `page_index` — 页码（可选）
    ///
    /// API: `GET /ws/place/v1/search`
    pub async fn search(
        &self,
        keyword: &str,
        boundary: &str,
        page_size: Option<&str>,
        page_index: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("keyword", keyword), ("boundary", boundary)];
        if let Some(ps) = page_size {
            params.push(("page_size", ps));
        }
        if let Some(pi) = page_index {
            params.push(("page_index", pi));
        }
        self.get("/ws/place/v1/search", &params).await
    }

    /// 多边形范围搜索
    ///
    /// - `keyword` — 搜索关键字
    /// - `polygon` — 多边形坐标，格式 "lat1,lng1;lat2,lng2;..."
    /// - `page_size` — 每页条数（可选）
    /// - `page_index` — 页码（可选）
    ///
    /// API: `GET /ws/place/v1/search_by_polygon`
    pub async fn search_by_polygon(
        &self,
        keyword: &str,
        polygon: &str,
        page_size: Option<&str>,
        page_index: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("keyword", keyword), ("polygon", polygon)];
        if let Some(ps) = page_size {
            params.push(("page_size", ps));
        }
        if let Some(pi) = page_index {
            params.push(("page_index", pi));
        }
        self.get("/ws/place/v1/search_by_polygon", &params).await
    }

    /// 周边推荐（explore）
    ///
    /// - `boundary` — 搜索范围，格式 "nearby(lat,lng,radius)"
    /// - `policy` — 搜索策略（可选）：1=签到场景, 2=位置共享场景
    ///
    /// API: `GET /ws/place/v1/explore`
    pub async fn explore(
        &self,
        boundary: &str,
        policy: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("boundary", boundary)];
        if let Some(p) = policy {
            params.push(("policy", p));
        }
        self.get("/ws/place/v1/explore", &params).await
    }

    /// 关键词输入提示
    ///
    /// - `keyword` — 搜索关键字
    /// - `region` — 指定城市（可选）
    ///
    /// API: `GET /ws/place/v1/suggestion`
    pub async fn suggestion(
        &self,
        keyword: &str,
        region: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![("keyword", keyword)];
        if let Some(r) = region {
            params.push(("region", r));
        }
        self.get("/ws/place/v1/suggestion", &params).await
    }

    /// POI 详情
    ///
    /// - `id` — POI 唯一标识，支持多个用逗号分隔（最多 10 个）
    ///
    /// API: `GET /ws/place/v1/detail`
    pub async fn poi_detail(&self, id: &str) -> crate::Result<TmapResponse> {
        self.get("/ws/place/v1/detail", &[("id", id)]).await
    }

    // ═══════════════════════════════════════════════════════════
    //  天气查询
    // ═══════════════════════════════════════════════════════════

    /// 天气查询
    ///
    /// - `adcode` — 行政区划代码（与 location 二选一），如 "110000"
    /// - `location` — 坐标（与 adcode 二选一），格式 "纬度,经度"
    /// - `type` — 查询类型：now(实时) / future(预报) / hours(逐小时)
    /// - `added_fields` — 附加字段：alarm / index / air（逗号分隔）
    ///
    /// API: `GET /ws/weather/v1/`
    pub async fn weather(
        &self,
        adcode: Option<&str>,
        location: Option<&str>,
        r#type: Option<&str>,
        added_fields: Option<&str>,
    ) -> crate::Result<TmapResponse> {
        let mut params = vec![];
        if let Some(a) = adcode {
            params.push(("adcode", a));
        }
        if let Some(l) = location {
            params.push(("location", l));
        }
        if let Some(t) = r#type {
            params.push(("type", t));
        }
        if let Some(af) = added_fields {
            params.push(("added_fields", af));
        }
        self.get("/ws/weather/v1/", &params).await
    }

    // ═══════════════════════════════════════════════════════════
    //  IP 定位
    // ═══════════════════════════════════════════════════════════

    /// IP 定位
    ///
    /// - `ip` — IP 地址（可选），不传则使用请求来源 IP
    ///
    /// API: `GET /ws/location/v1/ip`
    pub async fn ip_location(&self, ip: Option<&str>) -> crate::Result<TmapResponse> {
        let mut params = vec![];
        if let Some(i) = ip {
            params.push(("ip", i));
        }
        self.get("/ws/location/v1/ip", &params).await
    }

    // ═══════════════════════════════════════════════════════════
    //  坐标转换
    // ═══════════════════════════════════════════════════════════

    /// 坐标转换（其他坐标系 → 腾讯地图 GCJ-02）
    ///
    /// - `locations` — 坐标串，格式 "lat1,lng1;lat2,lng2"
    /// - `type` — 输入坐标系：1=GPS(WGS-84), 3=百度(BD-09), 4=MapBar
    ///
    /// API: `GET /ws/coord/v1/translate`
    pub async fn coord_translate(
        &self,
        locations: &str,
        r#type: &str,
    ) -> crate::Result<TmapResponse> {
        self.get(
            "/ws/coord/v1/translate",
            &[("locations", locations), ("type", r#type)],
        )
        .await
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "tmap")?;
        instance.client = reqwest::Client::new();
        Ok(instance)
    }
}
