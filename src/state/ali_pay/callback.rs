//! 支付宝异步通知处理
//!
//! 支付宝通过 POST form-urlencoded 方式发送异步通知。
//! 通知参数包含 trade_status 等字段，需要验签后分发到用户回调。

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  通知数据结构
// ═══════════════════════════════════════════════════════════════

/// 交易通知（支付成功等）
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AliPayTradeNotify {
    /// 通知时间
    #[serde(default)]
    pub notify_time: String,
    /// 通知类型
    #[serde(default)]
    pub notify_type: String,
    /// 通知校验 ID
    #[serde(default)]
    pub notify_id: String,
    /// 签名类型
    #[serde(default)]
    pub sign_type: String,
    /// 签名
    #[serde(default)]
    pub sign: String,
    /// 支付宝交易号
    #[serde(default)]
    pub trade_no: String,
    /// 商户订单号
    #[serde(default)]
    pub out_trade_no: String,
    /// 商户应用 ID
    #[serde(default)]
    pub app_id: String,
    /// 卖家支付宝用户号
    #[serde(default)]
    pub seller_id: String,
    /// 买家支付宝用户号
    #[serde(default)]
    pub buyer_id: String,
    /// 交易状态
    #[serde(default)]
    pub trade_status: String,
    /// 订单金额
    #[serde(default)]
    pub total_amount: String,
    /// 实收金额
    #[serde(default)]
    pub receipt_amount: String,
    /// 买家付款金额
    #[serde(default)]
    pub buyer_pay_amount: String,
    /// 退款金额
    #[serde(default)]
    pub refund_fee: String,
    /// 订单标题
    #[serde(default)]
    pub subject: String,
    /// 商品描述
    #[serde(default)]
    pub body: String,
    /// 交易创建时间
    #[serde(default)]
    pub gmt_create: String,
    /// 交易付款时间
    #[serde(default)]
    pub gmt_payment: String,
    /// 交易退款时间
    #[serde(default)]
    pub gmt_refund: String,
    /// 交易结束时间
    #[serde(default)]
    pub gmt_close: String,
    /// 支付渠道信息
    #[serde(default)]
    pub fund_channel_list: String,
    /// 买家用户类型
    #[serde(default)]
    pub buyer_user_type: String,
    /// 商家优惠金额
    #[serde(default)]
    pub mdiscount_amount: String,
    /// 平台优惠金额
    #[serde(default)]
    pub discount_amount: String,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 退款通知
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AliPayRefundNotify {
    /// 通知时间
    #[serde(default)]
    pub notify_time: String,
    /// 通知类型
    #[serde(default)]
    pub notify_type: String,
    /// 通知校验 ID
    #[serde(default)]
    pub notify_id: String,
    /// 签名
    #[serde(default)]
    pub sign: String,
    /// 签名类型
    #[serde(default)]
    pub sign_type: String,
    /// 商户应用 ID
    #[serde(default)]
    pub app_id: String,
    /// 支付宝交易号
    #[serde(default)]
    pub trade_no: String,
    /// 商户订单号
    #[serde(default)]
    pub out_trade_no: String,
    /// 退款请求号
    #[serde(default)]
    pub out_request_no: String,
    /// 银行卡冲退状态：S 成功 / F 失败
    #[serde(default)]
    pub dback_status: String,
    /// 银行卡冲退金额
    #[serde(default)]
    pub dback_amount: String,
    /// 银行响应时间
    #[serde(default)]
    pub bank_ack_time: String,
    /// 预估银行入账时间
    #[serde(default)]
    pub est_bank_receipt_time: String,
    /// 未识别的扩展字段
    #[serde(flatten)]
    #[serde(default)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// 通知处理结果
#[derive(Debug, Clone)]
pub struct AliPayNotifyResult {
    pub body: String,
}

impl AliPayNotifyResult {
    /// 返回 success（告诉支付宝停止重试）
    pub fn success() -> Self {
        Self {
            body: "success".to_string(),
        }
    }

    /// 返回 fail（支付宝会重试）
    pub fn fail() -> Self {
        Self {
            body: "fail".to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  通知分发
// ═══════════════════════════════════════════════════════════════

/// 将 form 参数解析为指定类型
fn parse_notify<T: for<'de> serde::Deserialize<'de>>(
    params: &[(String, String)],
) -> crate::Result<T> {
    let map: std::collections::HashMap<&str, &str> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    serde_json::from_value(serde_json::to_value(map).unwrap())
        .map_err(|e| super::err::notify_missing_params(&format!("parse: {}", e)))
}

/// 分发回调通知
pub(crate) async fn dispatch_notify(
    runtime: &super::AliPayRuntime,
    state: crate::AppState,
    params: Vec<(String, String)>,
) -> afast::Result<afast::Text> {
    // 1. 验签
    if let Some(pub_key) = &runtime.alipay_public_key_parsed {
        let sign = params
            .iter()
            .find(|(k, _)| k == "sign")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();

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

        let valid = super::rsa2_verify(&content, &sign, pub_key).unwrap_or(false);
        if !valid {
            return Ok(afast::Text("fail".to_string()));
        }
    }

    // 2. 判断通知类型并分发
    let notify_type = params
        .iter()
        .find(|(k, _)| k == "notify_type")
        .map(|(_, v)| v.as_str())
        .unwrap_or("");

    match notify_type {
        "trade_status_sync" => {
            let notify: AliPayTradeNotify = parse_notify(&params)?;
            if let Some(cb) = &runtime.pay_success_callback {
                let result = cb((state.clone(), notify)).await?;
                return Ok(afast::Text(result.body));
            }
        }
        "refund" | "refund.depositback.completed" => {
            let notify: AliPayRefundNotify = parse_notify(&params)?;
            if let Some(cb) = &runtime.refund_callback {
                let result = cb((state.clone(), notify)).await?;
                return Ok(afast::Text(result.body));
            }
        }
        _ => {
            // 未知通知类型，返回 success 避免重试
            #[cfg(feature = "log")]
            tracing::debug!(notify_type, "收到未知类型的支付宝通知");
        }
    }

    // 未注册回调时静默返回 success
    #[cfg(feature = "log")]
    tracing::debug!(notify_type, "支付宝通知未注册对应回调，静默返回 success");

    Ok(afast::Text("success".to_string()))
}

/// 支付宝异步通知表单体
///
/// 支付宝通过 POST form-urlencoded 发送，所有字段均为字符串。
/// 通知字段根据 notify_type 不同而不同，这里统一用 Option 捕获。
#[derive(Debug, Clone, Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("支付宝异步通知")]
pub struct AliPayNotifyBody {
    pub notify_time: Option<String>,
    pub notify_type: Option<String>,
    pub notify_id: Option<String>,
    pub sign_type: Option<String>,
    pub sign: Option<String>,
    pub trade_no: Option<String>,
    pub out_trade_no: Option<String>,
    pub app_id: Option<String>,
    pub seller_id: Option<String>,
    pub buyer_id: Option<String>,
    pub trade_status: Option<String>,
    pub total_amount: Option<String>,
    pub receipt_amount: Option<String>,
    pub buyer_pay_amount: Option<String>,
    pub refund_fee: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub gmt_create: Option<String>,
    pub gmt_payment: Option<String>,
    pub gmt_refund: Option<String>,
    pub gmt_close: Option<String>,
    pub out_request_no: Option<String>,
    pub dback_status: Option<String>,
    pub dback_amount: Option<String>,
    pub bank_ack_time: Option<String>,
    pub est_bank_receipt_time: Option<String>,
    pub fund_channel_list: Option<String>,
    pub buyer_user_type: Option<String>,
    pub mdiscount_amount: Option<String>,
    pub discount_amount: Option<String>,
}

/// 支付宝异步通知回调 handler
///
/// 框架自动注册为 POST 路由，接收 form-urlencoded 参数。
#[afast::post(desc("支付宝异步通知回调"))]
async fn callback(
    afast::State(state): afast::State<crate::AppState>,
    afast::Body(body): afast::Body<AliPayNotifyBody>,
) -> afast::Result<afast::Text> {
    // 转换为 Vec<(String, String)>
    let mut params: Vec<(String, String)> = Vec::new();
    macro_rules! push_field {
        ($field:ident) => {
            if let Some(v) = body.$field {
                if !v.is_empty() {
                    params.push((stringify!($field).to_string(), v));
                }
            }
        };
    }
    push_field!(notify_time);
    push_field!(notify_type);
    push_field!(notify_id);
    push_field!(sign_type);
    push_field!(sign);
    push_field!(trade_no);
    push_field!(out_trade_no);
    push_field!(app_id);
    push_field!(seller_id);
    push_field!(buyer_id);
    push_field!(trade_status);
    push_field!(total_amount);
    push_field!(receipt_amount);
    push_field!(buyer_pay_amount);
    push_field!(refund_fee);
    push_field!(subject);
    push_field!(body);
    push_field!(gmt_create);
    push_field!(gmt_payment);
    push_field!(gmt_refund);
    push_field!(gmt_close);
    push_field!(out_request_no);
    push_field!(dback_status);
    push_field!(dback_amount);
    push_field!(bank_ack_time);
    push_field!(est_bank_receipt_time);
    push_field!(fund_channel_list);
    push_field!(buyer_user_type);
    push_field!(mdiscount_amount);
    push_field!(discount_amount);

    if params.is_empty() {
        return Ok(afast::Text("fail".to_string()));
    }

    state
        .ali_pay
        .handle_notify(state.clone(), params)
        .await
        .map_err(|e| e.into())
}
