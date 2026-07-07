//! 支付宝电脑网站支付
//!
//! 提供：
//! - 统收单下单并支付 (`alipay.trade.page.pay`)
//! - 交易查询 (`alipay.trade.query`)
//! - 交易关闭 (`alipay.trade.close`)
//! - 退款 (`alipay.trade.refund`)
//! - 退款查询 (`alipay.trade.fastpay.refund.query`)
//! - 账单下载 (`alipay.data.dataservice.bill.downloadurl.query`)
//! - 异步通知验签与分发
//!
//! 签名算法：RSA2 (SHA256WithRSA)
//! API 网关：`https://openapi.alipay.com/gateway.do`

pub mod callback;
pub mod err;

pub use callback::{
    AliPayNotifyResult, AliPayRefundNotify, AliPayTradeNotify, callback as notify_handler,
};

use crate::state::callbacks::AsyncCallback;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2_v10::Digest;

/// API 网关地址
const GATEWAY_URL: &str = "https://openapi.alipay.com/gateway.do";

// ═══════════════════════════════════════════════════════════════
//  回调函数类型
// ═══════════════════════════════════════════════════════════════

/// 支付成功回调函数类型
pub type PaySuccessCallbackFn =
    AsyncCallback<(crate::AppState, AliPayTradeNotify), crate::Result<AliPayNotifyResult>>;

/// 退款回调函数类型
pub type RefundCallbackFn =
    AsyncCallback<(crate::AppState, AliPayRefundNotify), crate::Result<AliPayNotifyResult>>;

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

/// 支付宝配置
#[derive(Clone, Deserialize)]
pub struct AliPayConfig {
    /// 应用 ID
    pub app_id: String,
    /// 应用私钥 (PEM 格式)
    pub private_key: String,
    /// 支付宝公钥 (PEM 格式，用于验签)
    pub alipay_public_key: String,
    /// 异步通知回调完整 URL
    #[serde(default)]
    pub notify_url: Option<String>,
    /// 同步跳转 return_url
    #[serde(default)]
    pub return_url: Option<String>,
    /// API 网关地址，默认 `https://openapi.alipay.com/gateway.do`
    #[serde(default = "default_gateway")]
    pub gateway: String,
    /// 编码格式，默认 UTF-8
    #[serde(default = "default_charset")]
    pub charset: String,
    /// 签名类型，默认 RSA2
    #[serde(default = "default_sign_type")]
    pub sign_type: String,
}

fn default_gateway() -> String {
    GATEWAY_URL.to_string()
}
fn default_charset() -> String {
    "UTF-8".to_string()
}
fn default_sign_type() -> String {
    "RSA2".to_string()
}

/// 运行时状态
#[derive(Clone, Default)]
pub struct AliPayRuntime {
    pub(crate) client: reqwest::Client,
    pub(crate) private_key_parsed: Option<rsa::RsaPrivateKey>,
    pub(crate) alipay_public_key_parsed: Option<rsa::RsaPublicKey>,
    pub(crate) pay_success_callback: Option<PaySuccessCallbackFn>,
    pub(crate) refund_callback: Option<RefundCallbackFn>,
}

/// 支付宝电脑网站支付模块
#[derive(Clone, Deserialize)]
pub struct AliPay {
    #[serde(flatten)]
    pub config: AliPayConfig,
    /// 回调通知路由路径
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
    #[serde(skip)]
    pub(crate) runtime: AliPayRuntime,
}

fn default_callback_path() -> String {
    "ali/pay/notify".to_string()
}

// ═══════════════════════════════════════════════════════════════
//  请求 / 响应类型
// ═══════════════════════════════════════════════════════════════

/// 电脑网站支付 - 业务请求参数
#[derive(Debug, Clone, Serialize)]
pub struct PagePayRequest {
    /// 商户订单号 (64 字符以内)
    pub out_trade_no: String,
    /// 订单总金额，单位：元，精确到小数点后两位
    pub total_amount: String,
    /// 订单标题
    pub subject: String,
    /// 产品码，固定 FAST_INSTANT_TRADE_PAY
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_code: Option<String>,
    /// PC 扫码支付方式：0 简约前置 / 1 前置 / 2 跳转 / 3 迷你 / 4 自定义宽度
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_pay_mode: Option<String>,
    /// 二维码宽度（qr_pay_mode=4 时有效）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qrcode_width: Option<u64>,
    /// 订单绝对超时时间 yyyy-MM-dd HH:mm:ss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_expire: Option<String>,
    /// 请求后页面的集成方式 PCWEB / ALIAPP
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_type: Option<String>,
}

/// 交易查询 - 业务请求参数
#[derive(Debug, Clone, Serialize)]
pub struct TradeQueryRequest {
    /// 商户订单号（与 trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    /// 支付宝交易号（与 out_trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_no: Option<String>,
    /// 查询选项
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_options: Option<Vec<String>>,
}

/// 交易关闭 - 业务请求参数
#[derive(Debug, Clone, Serialize)]
pub struct TradeCloseRequest {
    /// 支付宝交易号（与 out_trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_no: Option<String>,
    /// 商户订单号（与 trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    /// 商家操作员编号
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator_id: Option<String>,
}

/// 退款 - 业务请求参数
#[derive(Debug, Clone, Serialize)]
pub struct RefundRequest {
    /// 退款金额，单位：元
    pub refund_amount: String,
    /// 商户订单号（与 trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    /// 支付宝交易号（与 out_trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_no: Option<String>,
    /// 退款原因
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refund_reason: Option<String>,
    /// 退款请求号（部分退款时必传）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_request_no: Option<String>,
}

/// 退款查询 - 业务请求参数
#[derive(Debug, Clone, Serialize)]
pub struct RefundQueryRequest {
    /// 退款请求号
    pub out_request_no: String,
    /// 支付宝交易号（与 out_trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_no: Option<String>,
    /// 商户订单号（与 trade_no 二选一）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_trade_no: Option<String>,
    /// 查询选项
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_options: Option<Vec<String>>,
}

/// 账单下载 - 业务请求参数
#[derive(Debug, Clone, Serialize)]
pub struct BillDownloadRequest {
    /// 账单类型：trade / signcustomer / merchant_act 等
    pub bill_type: String,
    /// 账单时间：日账单 yyyy-MM-dd / 月账单 yyyy-MM
    pub bill_date: String,
    /// 二级商户 smid（仅 bill_type=trade_zft_merchant 时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smid: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  通用响应
// ═══════════════════════════════════════════════════════════════

/// 支付宝 API 通用响应体
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AliPayResponse<T> {
    pub code: String,
    pub msg: String,
    pub sub_code: Option<String>,
    pub sub_msg: Option<String>,
    #[serde(flatten)]
    pub data: Option<T>,
}

/// 交易查询响应数据
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TradeQueryData {
    pub trade_no: Option<String>,
    pub out_trade_no: Option<String>,
    pub trade_status: Option<String>,
    pub total_amount: Option<String>,
    pub buyer_pay_amount: Option<String>,
    pub point_amount: Option<String>,
    pub invoice_amount: Option<String>,
    pub send_pay_date: Option<String>,
    pub receipt_amount: Option<String>,
    pub store_id: Option<String>,
    pub terminal_id: Option<String>,
    pub fund_bill_list: Option<serde_json::Value>,
    pub charge_amount: Option<String>,
    pub charge_flags: Option<String>,
    pub settlement_id: Option<String>,
    pub trade_settle_info: Option<serde_json::Value>,
    pub discount_goods_detail: Option<String>,
    pub buyer_user_id: Option<String>,
    pub mdiscount_amount: Option<String>,
    pub discount_amount: Option<String>,
    pub buyer_user_type: Option<String>,
    pub mdstore_merchant_type: Option<String>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 交易关闭响应数据
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct TradeCloseData {
    pub trade_no: Option<String>,
    pub out_trade_no: Option<String>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 退款响应数据
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RefundData {
    pub trade_no: Option<String>,
    pub out_trade_no: Option<String>,
    pub buyer_logon_id: Option<String>,
    pub fund_change: Option<String>,
    pub refund_fee: Option<String>,
    pub gmt_refund_pay: Option<String>,
    pub refund_detail_item_list: Option<serde_json::Value>,
    pub store_name: Option<String>,
    pub buyer_user_id: Option<String>,
    pub send_back_fee: Option<String>,
    pub refund_preset_paytool_list: Option<serde_json::Value>,
    pub refund_charge_amount: Option<String>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 退款查询响应数据
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RefundQueryData {
    pub trade_no: Option<String>,
    pub out_trade_no: Option<String>,
    pub out_request_no: Option<String>,
    pub refund_status: Option<String>,
    pub total_amount: Option<String>,
    pub refund_amount: Option<String>,
    pub refund_royaltys: Option<serde_json::Value>,
    pub gmt_refund_pay: Option<String>,
    pub refund_detail_item_list: Option<serde_json::Value>,
    pub send_back_fee: Option<String>,
    pub deposit_back_info: Option<serde_json::Value>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 账单下载响应数据
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BillDownloadData {
    pub bill_download_url: Option<String>,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

// ═══════════════════════════════════════════════════════════════
//  签名 & 验签
// ═══════════════════════════════════════════════════════════════

/// 加载 PEM 格式 RSA 私钥
fn load_private_key(pem_str: &str) -> crate::Result<rsa::RsaPrivateKey> {
    use rsa::pkcs8::DecodePrivateKey;
    let pem =
        pem::parse(pem_str).map_err(|e| err::private_key_load(&format!("PEM parse: {}", e)))?;
    rsa::RsaPrivateKey::from_pkcs8_der(pem.contents())
        .map_err(|e| err::private_key_load(&format!("PKCS8 decode: {}", e)))
}

/// 加载 PEM 格式 RSA 公钥（支付宝公钥）
fn load_public_key(pem_str: &str) -> crate::Result<rsa::RsaPublicKey> {
    use rsa::pkcs8::DecodePublicKey;
    let pem =
        pem::parse(pem_str).map_err(|e| err::public_key_load(&format!("PEM parse: {}", e)))?;
    rsa::RsaPublicKey::from_public_key_der(pem.contents())
        .map_err(|e| err::public_key_load(&format!("DER decode: {}", e)))
}

/// RSA2 (SHA256WithRSA) 签名
///
/// 将请求参数按 key 的 ASCII 升序排列，拼接为 `key=value&key=value`，用私钥签名，Base64 编码。
fn rsa2_sign(
    params: &[(String, String)],
    private_key: &rsa::RsaPrivateKey,
) -> afast::Result<String> {
    let mut sorted: Vec<(&str, &str)> = params
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    sorted.sort_by_key(|(k, _)| *k);
    let sign_content = sorted
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let mut hasher = sha2_v10::Sha256::new();
    hasher.update(sign_content.as_bytes());
    let hash = hasher.finalize();

    let padding = rsa::Pkcs1v15Sign::new::<sha2_v10::Sha256>();
    let signature = private_key
        .sign(padding, &hash)
        .map_err(|e| err::sign_failed(&format!("{}", e)))?;

    Ok(base64::engine::general_purpose::STANDARD.encode(&signature))
}

/// RSA2 验签
///
/// 将响应参数按 key 升序排列拼接，用支付宝公钥验签。
fn rsa2_verify(content: &str, sign: &str, public_key: &rsa::RsaPublicKey) -> crate::Result<bool> {
    let sign_bytes = base64::engine::general_purpose::STANDARD
        .decode(sign)
        .map_err(|e| err::verify_failed(&format!("base64 decode: {}", e)))?;

    let mut hasher = sha2_v10::Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    let padding = rsa::Pkcs1v15Sign::new::<sha2_v10::Sha256>();
    Ok(public_key.verify(padding, &hash, &sign_bytes).is_ok())
}

/// 构造待签名/验签的有序字符串
///
/// 从 JSON 对象中提取所有顶层 key，按 ASCII 升序排列，拼接为 `key=value&key=value`。
/// 排除 `sign` 字段本身和空值。
fn build_sign_content_from_json(json_str: &str) -> afast::Result<String> {
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(json_str)
        .map_err(|e| err::sign_failed(&format!("JSON parse: {}", e)))?;
    let mut pairs: Vec<(&str, &String)> = map
        .iter()
        .filter(|(k, _)| k.as_str() != "sign")
        .filter_map(|(k, v)| {
            if let serde_json::Value::String(s) = v
                && !s.is_empty()
            {
                return Some((k.as_str(), s));
            }
            None
        })
        .collect();
    pairs.sort_by_key(|(k, _)| *k);
    Ok(pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&"))
}

// ═══════════════════════════════════════════════════════════════
//  公共参数构造
// ═══════════════════════════════════════════════════════════════

fn build_common_params(
    config: &AliPayConfig,
    method: &str,
    biz_content: &str,
    timestamp: &str,
) -> Vec<(String, String)> {
    let mut params = vec![
        ("app_id".to_string(), config.app_id.clone()),
        ("method".to_string(), method.to_string()),
        ("format".to_string(), "JSON".to_string()),
        ("charset".to_string(), config.charset.clone()),
        ("sign_type".to_string(), config.sign_type.clone()),
        ("timestamp".to_string(), timestamp.to_string()),
        ("version".to_string(), "1.0".to_string()),
        ("biz_content".to_string(), biz_content.to_string()),
    ];
    if let Some(ref notify_url) = config.notify_url {
        params.push(("notify_url".to_string(), notify_url.clone()));
    }
    if let Some(ref return_url) = config.return_url {
        params.push(("return_url".to_string(), return_url.clone()));
    }
    params
}

/// 调用支付宝 API（POST form-urlencoded）
async fn call_api<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    config: &AliPayConfig,
    private_key: &rsa::RsaPrivateKey,
    alipay_public_key: Option<&rsa::RsaPublicKey>,
    method_name: &str,
    biz_content: &str,
    response_key: &str,
) -> crate::Result<T> {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut params = build_common_params(config, method_name, biz_content, &now);
    let sign = rsa2_sign(&params, private_key)?;
    params.push(("sign".to_string(), sign));

    let form: std::collections::HashMap<&str, &str> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let resp = client
        .post(&config.gateway)
        .form(&form)
        .send()
        .await
        .map_err(|e| err::request_failed(&format!("{}", e)))?;

    let body = resp
        .text()
        .await
        .map_err(|e| err::response_parse_failed(&format!("read body: {}", e)))?;

    let outer: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| err::response_parse_failed(&format!("JSON parse: {}", e)))?;

    // 验签
    if let (Some(sign_val), Some(pub_key)) = (
        outer.get("sign").and_then(|v| v.as_str()),
        alipay_public_key,
    ) && let Some(resp_obj) = outer.get(response_key)
        && let Ok(sign_content) = build_sign_content_from_json(&resp_obj.to_string())
    {
        let _ = rsa2_verify(&sign_content, sign_val, pub_key);
    }

    let resp_value = outer.get(response_key).unwrap_or(&outer);
    serde_json::from_value(resp_value.clone())
        .map_err(|e| err::response_parse_failed(&format!("deserialize: {} (body: {})", e, body)))
}

// ═══════════════════════════════════════════════════════════════
//  AliPay 实现
// ═══════════════════════════════════════════════════════════════

impl AliPay {
    /// 初始化运行时（加载私钥/公钥、创建 HTTP 客户端）
    pub fn init(&mut self) -> afast::Result<()> {
        self.runtime.client = super::default_http_client();
        self.runtime.private_key_parsed = Some(load_private_key(&self.config.private_key)?);
        if !self.config.alipay_public_key.is_empty() {
            self.runtime.alipay_public_key_parsed =
                Some(load_public_key(&self.config.alipay_public_key)?);
        }
        Ok(())
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "ali_pay")?;
        instance.init()?;
        Ok(instance)
    }

    /// 注册支付成功回调
    pub fn with_pay_success_callback(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<
            (crate::AppState, AliPayTradeNotify),
            crate::Result<AliPayNotifyResult>,
        >,
    ) -> Self {
        self.runtime.pay_success_callback = Some(cb.into_callback());
        self
    }

    /// 注册退款回调
    pub fn with_refund_callback(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<
            (crate::AppState, AliPayRefundNotify),
            crate::Result<AliPayNotifyResult>,
        >,
    ) -> Self {
        self.runtime.refund_callback = Some(cb.into_callback());
        self
    }

    // ── 电脑网站支付下单 ──

    /// 生成电脑网站支付表单 HTML
    ///
    /// 返回一个自动提交的 HTML 表单，浏览器渲染后会自动跳转到支付宝收银台。
    /// 适用于 GET/POST 两种方式。
    pub fn page_pay(&self, req: &PagePayRequest) -> afast::Result<String> {
        let biz_content = serde_json::to_string(req)
            .map_err(|e| err::sign_failed(&format!("serialize: {}", e)))?;
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let mut params =
            build_common_params(&self.config, "alipay.trade.page.pay", &biz_content, &now);
        let sign = rsa2_sign(
            &params,
            self.runtime
                .private_key_parsed
                .as_ref()
                .ok_or_else(err::private_key_not_init)?,
        )?;
        params.push(("sign".to_string(), sign));

        // 生成自动提交表单
        let inputs: String = params
            .iter()
            .map(|(k, v)| {
                format!(
                    r#"<input type="hidden" name="{}" value="{}" />"#,
                    k,
                    v.replace('&', "&amp;")
                        .replace('"', "&quot;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            r#"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>正在跳转到支付宝...</title></head>
<body>
<form id="alipay_form" action="{}" method="POST">
{}
<button type="submit">正在跳转到支付宝...</button>
</form>
<script>document.getElementById('alipay_form').submit();</script>
</body>
</html>"#,
            self.config.gateway, inputs
        ))
    }

    // ── 交易查询 ──

    pub async fn query_trade(
        &self,
        out_trade_no: Option<&str>,
        trade_no: Option<&str>,
    ) -> crate::Result<AliPayResponse<TradeQueryData>> {
        let req = TradeQueryRequest {
            out_trade_no: out_trade_no.map(|s| s.to_string()),
            trade_no: trade_no.map(|s| s.to_string()),
            query_options: None,
        };
        let biz = serde_json::to_string(&req)
            .map_err(|e| err::request_failed(&format!("serialize: {}", e)))?;
        call_api(
            &self.runtime.client,
            &self.config,
            self.runtime
                .private_key_parsed
                .as_ref()
                .ok_or_else(err::private_key_not_init)?,
            self.runtime.alipay_public_key_parsed.as_ref(),
            "alipay.trade.query",
            &biz,
            "alipay_trade_query_response",
        )
        .await
    }

    // ── 交易关闭 ──

    pub async fn close_trade(
        &self,
        out_trade_no: Option<&str>,
        trade_no: Option<&str>,
    ) -> crate::Result<AliPayResponse<TradeCloseData>> {
        let req = TradeCloseRequest {
            trade_no: trade_no.map(|s| s.to_string()),
            out_trade_no: out_trade_no.map(|s| s.to_string()),
            operator_id: None,
        };
        let biz = serde_json::to_string(&req)
            .map_err(|e| err::request_failed(&format!("serialize: {}", e)))?;
        call_api(
            &self.runtime.client,
            &self.config,
            self.runtime
                .private_key_parsed
                .as_ref()
                .ok_or_else(err::private_key_not_init)?,
            self.runtime.alipay_public_key_parsed.as_ref(),
            "alipay.trade.close",
            &biz,
            "alipay_trade_close_response",
        )
        .await
    }

    // ── 退款 ──

    pub async fn refund(
        &self,
        refund_amount: &str,
        out_trade_no: Option<&str>,
        trade_no: Option<&str>,
        refund_reason: Option<&str>,
        out_request_no: Option<&str>,
    ) -> crate::Result<AliPayResponse<RefundData>> {
        let req = RefundRequest {
            refund_amount: refund_amount.to_string(),
            out_trade_no: out_trade_no.map(|s| s.to_string()),
            trade_no: trade_no.map(|s| s.to_string()),
            refund_reason: refund_reason.map(|s| s.to_string()),
            out_request_no: out_request_no.map(|s| s.to_string()),
        };
        let biz = serde_json::to_string(&req)
            .map_err(|e| err::request_failed(&format!("serialize: {}", e)))?;
        call_api(
            &self.runtime.client,
            &self.config,
            self.runtime
                .private_key_parsed
                .as_ref()
                .ok_or_else(err::private_key_not_init)?,
            self.runtime.alipay_public_key_parsed.as_ref(),
            "alipay.trade.refund",
            &biz,
            "alipay_trade_refund_response",
        )
        .await
    }

    // ── 退款查询 ──

    pub async fn query_refund(
        &self,
        out_request_no: &str,
        out_trade_no: Option<&str>,
        trade_no: Option<&str>,
    ) -> crate::Result<AliPayResponse<RefundQueryData>> {
        let req = RefundQueryRequest {
            out_request_no: out_request_no.to_string(),
            trade_no: trade_no.map(|s| s.to_string()),
            out_trade_no: out_trade_no.map(|s| s.to_string()),
            query_options: None,
        };
        let biz = serde_json::to_string(&req)
            .map_err(|e| err::request_failed(&format!("serialize: {}", e)))?;
        call_api(
            &self.runtime.client,
            &self.config,
            self.runtime
                .private_key_parsed
                .as_ref()
                .ok_or_else(err::private_key_not_init)?,
            self.runtime.alipay_public_key_parsed.as_ref(),
            "alipay.trade.fastpay.refund.query",
            &biz,
            "alipay_trade_fastpay_refund_query_response",
        )
        .await
    }

    // ── 账单下载地址 ──

    pub async fn query_bill_download_url(
        &self,
        bill_type: &str,
        bill_date: &str,
    ) -> crate::Result<AliPayResponse<BillDownloadData>> {
        let req = BillDownloadRequest {
            bill_type: bill_type.to_string(),
            bill_date: bill_date.to_string(),
            smid: None,
        };
        let biz = serde_json::to_string(&req)
            .map_err(|e| err::request_failed(&format!("serialize: {}", e)))?;
        call_api(
            &self.runtime.client,
            &self.config,
            self.runtime
                .private_key_parsed
                .as_ref()
                .ok_or_else(err::private_key_not_init)?,
            self.runtime.alipay_public_key_parsed.as_ref(),
            "alipay.data.dataservice.bill.downloadurl.query",
            &biz,
            "alipay_data_dataservice_bill_downloadurl_query_response",
        )
        .await
    }

    // ── 验签（供回调使用） ──

    /// 验证支付宝异步通知签名
    ///
    /// 从 form 参数中提取所有非 `sign`、非空字段，按 key 升序拼接，
    /// 使用支付宝公钥验证 `sign` 字段。
    pub fn verify_notify_sign(&self, params: &[(String, String)]) -> crate::Result<bool> {
        let pub_key = self
            .runtime
            .alipay_public_key_parsed
            .as_ref()
            .ok_or_else(err::public_key_not_configured)?;

        let sign = params
            .iter()
            .find(|(k, _)| k == "sign")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

        let sign_type = params
            .iter()
            .find(|(k, _)| k == "sign_type")
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "RSA2".to_string());

        if sign_type != "RSA2" {
            return Err(err::unsupported_sign_type(&sign_type));
        }

        let mut pairs: Vec<(&str, &str)> = params
            .iter()
            .filter(|(k, _)| k != "sign" && k != "sign_type")
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        pairs.sort_by_key(|(k, _)| *k);
        let content = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        rsa2_verify(&content, &sign, pub_key)
    }

    /// 处理异步通知（验签 → 分发）
    pub async fn handle_notify(
        &self,
        state: crate::AppState,
        params: Vec<(String, String)>,
    ) -> afast::Result<afast::Text> {
        callback::dispatch_notify(&self.runtime, state, params).await
    }
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 支付宝配置扩展
pub trait AFasterAliPayExt {
    /// 链式配置支付宝电脑网站支付
    fn with_ali_pay(self, f: impl FnOnce(AliPay) -> AliPay) -> Self;
}

impl AFasterAliPayExt for crate::AFaster {
    fn with_ali_pay(mut self, f: impl FnOnce(AliPay) -> AliPay) -> Self {
        self.state.ali_pay = f(self.state.ali_pay);
        self
    }
}
