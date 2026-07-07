pub(crate) mod err;
use err::*;

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

// ═══════════════════════════════════════════════════════════════
//  高德地图 Web 服务
// ═══════════════════════════════════════════════════════════════

const AMAP_BASE: &str = "https://restapi.amap.com";

/// 高德地图 Web 服务 API 模块
///
/// 支持：地理/逆地理编码、路径规划、POI 搜索、天气查询
///
/// API: https://lbs.amap.com/api/webservice/summary
#[derive(Clone, Deserialize)]
pub struct Amap {
    /// Web 服务 API Key
    pub key: String,
    #[serde(skip)]
    pub client: Client,
}

// ── 通用响应 ──────────────────────────────────────────────────

/// 高德地图通用响应结构
///
/// 兼容 v3 和 v4 API：
/// - v3: status / info / infocode
/// - v4: success / errcode / errmsg
///
/// 业务数据通过 `data` 获取（`#[serde(flatten)]` 自动收集剩余字段）。
#[derive(Debug, Deserialize)]
pub struct AmapResponse {
    // ── v3 字段 ──
    /// "1" 表示成功，"0" 表示失败（v3 API）
    #[serde(default)]
    pub status: String,
    /// 成功时为 "OK"，失败时为错误描述（v3 API）
    #[serde(default)]
    pub info: String,
    /// 错误码，成功时为 "10000"（v3 API）
    #[serde(default)]
    pub infocode: String,

    // ── v4 字段 ──
    /// 错误码（v4 API），0 表示成功
    #[serde(default)]
    pub errcode: Option<i64>,
    /// 错误描述（v4 API）
    #[serde(default)]
    pub errmsg: Option<String>,

    /// 业务数据（geocodes / regeocode / route / pois / lives / forecasts / data 等）
    #[serde(flatten)]
    pub data: Value,
}

impl AmapResponse {
    /// 请求是否成功（兼容 v3 和 v4）
    #[inline]
    pub fn is_ok(&self) -> bool {
        // v3 格式: status == "1"
        if self.status == "1" {
            return true;
        }
        // v4 格式: errcode == 0 表示成功，无 errcode 也视为成功（纯 data 响应）
        if let Some(code) = self.errcode {
            return code == 0;
        }
        // 如果没有 status 也没有 errcode，检查是否有业务数据
        !self.status.is_empty() || self.data.as_object().is_some_and(|m| !m.is_empty())
    }

    /// 获取错误信息（兼容 v3 和 v4）
    pub fn error_info(&self) -> String {
        // v3
        if !self.info.is_empty() {
            return format!("{} ({})", self.info, self.infocode);
        }
        // v4
        if let Some(ref msg) = self.errmsg {
            let code = self.errcode.unwrap_or(-1);
            return format!("{} ({})", msg, code);
        }
        "unknown error".to_string()
    }
}

// ── 实现 ──────────────────────────────────────────────────────

impl Default for Amap {
    fn default() -> Self {
        Self::new()
    }
}

impl Amap {
    pub fn new() -> Self {
        Amap {
            key: String::new(),
            client: Client::new(),
        }
    }

    // ── 内部通用请求 ──────────────────────────────────────────

    /// 发送 GET 请求到高德 API
    async fn get(&self, path: &str, params: &[(&str, &str)]) -> crate::Result<AmapResponse> {
        if self.key.is_empty() {
            return Err(missing_key());
        }

        let url = format!("{}{}", AMAP_BASE, path);

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
                tracing::error!({ code = 51301, msg = "AMap request failed" }, "{}", e);
                request_failed(&e.to_string())
            })?;

        let text = resp.text().await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51302, msg = "AMap response read failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        let amap_resp: AmapResponse = serde_json::from_str(&text).map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51302, msg = "AMap response parse failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        if !amap_resp.is_ok() {
            let err_msg = amap_resp.error_info();
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 51303, msg = "AMap API error" },
                "{}", err_msg
            );
            return Err(api_error(&err_msg, ""));
        }

        Ok(amap_resp)
    }

    // ═══════════════════════════════════════════════════════════
    //  地理编码 / 逆地理编码
    // ═══════════════════════════════════════════════════════════

    /// 地理编码：结构化地址 → 经纬度坐标
    ///
    /// - `address` — 结构化地址，如 "北京市朝阳区阜通东大街6号"
    /// - `city` — 指定城市（可选），如 "北京"
    ///
    /// API: `GET /v3/geocode/geo`
    pub async fn geocode(&self, address: &str, city: Option<&str>) -> crate::Result<AmapResponse> {
        let mut params = vec![("address", address)];
        if let Some(c) = city {
            params.push(("city", c));
        }
        self.get("/v3/geocode/geo", &params).await
    }

    /// 逆地理编码：经纬度坐标 → 结构化地址
    ///
    /// - `location` — 经纬度，格式 "经度,纬度"，如 "116.480881,39.989410"
    /// - `extensions` — "base"（默认）或 "all"（含 POI、道路等）
    /// - `radius` — 搜索半径，0~3000 米，默认 1000
    ///
    /// API: `GET /v3/geocode/regeo`
    pub async fn regeocode(
        &self,
        location: &str,
        extensions: Option<&str>,
        radius: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("location", location)];
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        if let Some(r) = radius {
            params.push(("radius", r));
        }
        self.get("/v3/geocode/regeo", &params).await
    }

    // ═══════════════════════════════════════════════════════════
    //  路径规划
    // ═══════════════════════════════════════════════════════════

    /// 步行路径规划（最大 100km）
    ///
    /// - `origin` — 出发点，格式 "经度,纬度"
    /// - `destination` — 目的地，格式 "经度,纬度"
    ///
    /// API: `GET /v3/direction/walking`
    pub async fn direction_walking(
        &self,
        origin: &str,
        destination: &str,
    ) -> crate::Result<AmapResponse> {
        self.get(
            "/v3/direction/walking",
            &[("origin", origin), ("destination", destination)],
        )
        .await
    }

    /// 驾车路径规划
    ///
    /// - `origin` — 出发点，格式 "经度,纬度"
    /// - `destination` — 目的地，格式 "经度,纬度"
    /// - `strategy` — 驾车策略（可选），0~20，参见文档
    /// - `extensions` — "base" 或 "all"（含详细路段）
    ///
    /// API: `GET /v3/direction/driving`
    pub async fn direction_driving(
        &self,
        origin: &str,
        destination: &str,
        strategy: Option<&str>,
        extensions: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("origin", origin), ("destination", destination)];
        if let Some(s) = strategy {
            params.push(("strategy", s));
        }
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        self.get("/v3/direction/driving", &params).await
    }

    /// 公交路径规划
    ///
    /// - `origin` — 出发点，格式 "经度,纬度"
    /// - `destination` — 目的地，格式 "经度,纬度"
    /// - `city` — 起点城市，如 "北京" 或 "010"
    /// - `strategy` — 公交策略（可选）：0 最快捷，1 最经济，2 最少换乘，3 最少步行，5 不乘地铁
    /// - `extensions` — "base" 或 "all"
    ///
    /// API: `GET /v3/direction/transit/integrated`
    pub async fn direction_transit(
        &self,
        origin: &str,
        destination: &str,
        city: &str,
        strategy: Option<&str>,
        extensions: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![
            ("origin", origin),
            ("destination", destination),
            ("city", city),
        ];
        if let Some(s) = strategy {
            params.push(("strategy", s));
        }
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        self.get("/v3/direction/transit/integrated", &params).await
    }

    /// 骑行路径规划（最大 500km）
    ///
    /// - `origin` — 出发点，格式 "经度,纬度"
    /// - `destination` — 目的地，格式 "经度,纬度"
    ///
    /// API: `GET /v4/direction/bicycling`
    pub async fn direction_bicycling(
        &self,
        origin: &str,
        destination: &str,
    ) -> crate::Result<AmapResponse> {
        self.get(
            "/v4/direction/bicycling",
            &[("origin", origin), ("destination", destination)],
        )
        .await
    }

    /// 距离测量（支持批量）
    ///
    /// - `origins` — 出发点，多个用 "|" 分隔，最多 100 个，格式 "经度,纬度"
    /// - `destination` — 目的地，格式 "经度,纬度"
    /// - `type` — 计算方式：0 直线距离，1 驾车导航距离，3 步行距离
    ///
    /// API: `GET /v3/distance`
    pub async fn distance(
        &self,
        origins: &str,
        destination: &str,
        r#type: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("origins", origins), ("destination", destination)];
        if let Some(t) = r#type {
            params.push(("type", t));
        }
        self.get("/v3/distance", &params).await
    }

    // ═══════════════════════════════════════════════════════════
    //  POI 搜索
    // ═══════════════════════════════════════════════════════════

    /// POI 关键字搜索
    ///
    /// - `keywords` — 关键字，如 "北京大学"
    /// - `city` — 城市（可选），如 "北京"
    /// - `citylimit` — 是否限制在城市内（可选）
    /// - `offset` — 每页条数（可选，建议 ≤25）
    /// - `page` — 页码（可选）
    /// - `extensions` — "base" 或 "all"
    ///
    /// API: `GET /v3/place/text`
    pub async fn poi_text(
        &self,
        keywords: &str,
        city: Option<&str>,
        citylimit: Option<&str>,
        offset: Option<&str>,
        page: Option<&str>,
        extensions: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("keywords", keywords)];
        if let Some(c) = city {
            params.push(("city", c));
        }
        if let Some(cl) = citylimit {
            params.push(("citylimit", cl));
        }
        if let Some(o) = offset {
            params.push(("offset", o));
        }
        if let Some(p) = page {
            params.push(("page", p));
        }
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        self.get("/v3/place/text", &params).await
    }

    /// POI 周边搜索
    ///
    /// - `location` — 中心点，格式 "经度,纬度"
    /// - `keywords` — 关键字（可选）
    /// - `types` — POI 类型（可选），多个用 "|" 分隔
    /// - `radius` — 搜索半径，0~50000 米（可选，默认 5000）
    /// - `offset` — 每页条数（可选）
    /// - `page` — 页码（可选）
    /// - `sortrule` — 排序：distance 或 weight（可选）
    /// - `extensions` — "base" 或 "all"
    ///
    /// API: `GET /v3/place/around`
    pub async fn poi_around(
        &self,
        location: &str,
        keywords: Option<&str>,
        types: Option<&str>,
        radius: Option<&str>,
        offset: Option<&str>,
        page: Option<&str>,
        sortrule: Option<&str>,
        extensions: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("location", location)];
        if let Some(k) = keywords {
            params.push(("keywords", k));
        }
        if let Some(t) = types {
            params.push(("types", t));
        }
        if let Some(r) = radius {
            params.push(("radius", r));
        }
        if let Some(o) = offset {
            params.push(("offset", o));
        }
        if let Some(p) = page {
            params.push(("page", p));
        }
        if let Some(s) = sortrule {
            params.push(("sortrule", s));
        }
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        self.get("/v3/place/around", &params).await
    }

    /// POI 多边形搜索
    ///
    /// - `polygon` — 多边形坐标，格式 "经度,纬度|经度,纬度|..."，首尾需相同
    /// - `keywords` — 关键字（可选）
    /// - `types` — POI 类型（可选）
    /// - `offset` — 每页条数（可选）
    /// - `page` — 页码（可选）
    /// - `extensions` — "base" 或 "all"
    ///
    /// API: `GET /v3/place/polygon`
    pub async fn poi_polygon(
        &self,
        polygon: &str,
        keywords: Option<&str>,
        types: Option<&str>,
        offset: Option<&str>,
        page: Option<&str>,
        extensions: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("polygon", polygon)];
        if let Some(k) = keywords {
            params.push(("keywords", k));
        }
        if let Some(t) = types {
            params.push(("types", t));
        }
        if let Some(o) = offset {
            params.push(("offset", o));
        }
        if let Some(p) = page {
            params.push(("page", p));
        }
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        self.get("/v3/place/polygon", &params).await
    }

    /// POI ID 查询
    ///
    /// - `id` — POI 唯一标识
    ///
    /// API: `GET /v3/place/detail`
    pub async fn poi_detail(&self, id: &str) -> crate::Result<AmapResponse> {
        self.get("/v3/place/detail", &[("id", id)]).await
    }

    // ═══════════════════════════════════════════════════════════
    //  天气查询
    // ═══════════════════════════════════════════════════════════

    /// 天气查询
    ///
    /// - `city` — 城市编码，如 "110000"（北京）
    /// - `extensions` — "base"（实况天气，默认）或 "all"（预报天气）
    ///
    /// API: `GET /v3/weather/weatherInfo`
    pub async fn weather(
        &self,
        city: &str,
        extensions: Option<&str>,
    ) -> crate::Result<AmapResponse> {
        let mut params = vec![("city", city)];
        if let Some(ext) = extensions {
            params.push(("extensions", ext));
        }
        self.get("/v3/weather/weatherInfo", &params).await
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "amap")?;
        instance.client = super::default_http_client();
        Ok(instance)
    }
}
