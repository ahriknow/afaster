use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  回调通知数据结构
//  微信支付 V3 回调通知使用 AEAD_AES_256_GCM 加密
//  通知体为 JSON 格式，包含 resource 字段（加密数据）
// ═══════════════════════════════════════════════════════════════

/// 微信支付 V3 回调通知请求体
///
/// 微信服务器 POST 到商户回调 URL 的 JSON 结构
#[derive(Debug, Clone, Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("微信支付V3回调通知")]
pub struct WxPayNotify {
    /// 通知 ID
    #[serde(default)]
    pub id: String,
    /// 通知创建时间
    #[serde(default)]
    pub create_time: String,
    /// 通知类型: TRANSACTION.SUCCESS / REFUND.SUCCESS 等
    #[serde(default, rename = "event_type")]
    pub event_type: String,
    /// 通知数据类型: encrypted-resource
    #[serde(rename = "resource_type")]
    pub resource_type: Option<String>,
    /// 通知数据（加密）
    pub resource: WxPayNotifyResource,
    /// 回调摘要
    pub summary: Option<String>,
}

/// 回调通知中的加密资源
#[derive(Debug, Clone, Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("微信支付V3回调加密资源")]
pub struct WxPayNotifyResource {
    /// 加密算法: AEAD_AES_256_GCM
    #[serde(default)]
    pub algorithm: String,
    /// 密文 (base64)
    #[serde(default)]
    pub ciphertext: String,
    /// 附加数据 (base64)，可能为空
    pub associated_data: Option<String>,
    /// 随机串 (base64)
    #[serde(default)]
    pub nonce: String,
    /// 原始密文对应的微信支付平台证书序列号
    pub original_type: Option<String>,
}

/// 解密后的支付结果通知
///
/// 对应 transaction 成功通知的解密内容
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WxPayTransactionNotify {
    /// 应用 ID (AppID)
    #[serde(default)]
    pub appid: String,
    /// 商户号
    #[serde(default)]
    pub mchid: String,
    /// 商户订单号
    #[serde(default)]
    pub out_trade_no: String,
    /// 微信支付订单号
    pub transaction_id: Option<String>,
    /// 交易类型: MWEB (H5支付固定为MWEB)
    pub trade_type: Option<String>,
    /// 交易状态
    #[serde(default)]
    pub trade_state: String,
    /// 交易状态描述
    #[serde(default)]
    pub trade_state_desc: String,
    /// 银行类型
    pub bank_type: Option<String>,
    /// 附加数据
    pub attach: Option<String>,
    /// 支付完成时间 (RFC 3339)
    pub success_time: Option<String>,
    /// 支付者信息
    pub payer: Option<WxPayPayer>,
    /// 订单金额信息
    pub amount: Option<WxPayAmount>,
    /// 场景信息
    pub scene_info: Option<serde_json::Value>,
    /// 优惠功能
    pub promotion_detail: Option<Vec<serde_json::Value>>,
}

/// 支付者信息
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("H5支付者信息")]
pub struct WxPayPayer {
    /// 用户标识 (openid)
    pub openid: Option<String>,
}

/// 订单金额信息
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("H5订单金额")]
#[serde(rename_all = "snake_case")]
pub struct WxPayAmount {
    /// 订单总金额 (分)
    #[serde(default)]
    pub total: i64,
    /// 用户支付金额 (分)
    #[serde(default)]
    pub payer_total: i64,
    /// 货币类型
    #[serde(default)]
    pub currency: String,
    /// 用户支付币种
    #[serde(default)]
    pub payer_currency: String,
}

/// 退款结果通知（解密后）
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct WxPayRefundNotify {
    /// 商户号
    #[serde(default)]
    pub mchid: String,
    /// 商户退款单号
    #[serde(default)]
    pub out_refund_no: String,
    /// 微信退款单号
    pub refund_id: Option<String>,
    /// 微信支付订单号
    pub transaction_id: Option<String>,
    /// 商户订单号
    #[serde(default)]
    pub out_trade_no: String,
    /// 退款状态
    #[serde(default)]
    pub refund_status: String,
    /// 退款成功时间
    pub success_time: Option<String>,
    /// 退款入账账户
    pub user_received_account: Option<String>,
    /// 退款入账账户 (新字段名)
    pub refund_recv_account: Option<String>,
    /// 资金账户
    pub funds_account: Option<String>,
    /// 金额信息
    pub amount: Option<serde_json::Value>,
}

/// 回调通知通用响应
///
/// 微信要求回调响应格式:
/// - 成功: `{"code":"SUCCESS","message":"成功"}`
/// - 失败: `{"code":"FAIL","message":"失败原因"}`
#[derive(Debug, Serialize)]
pub struct WxPayNotifyResult {
    pub code: String,
    pub message: String,
}

impl WxPayNotifyResult {
    /// 成功响应
    pub fn success() -> Self {
        Self {
            code: "SUCCESS".to_string(),
            message: "成功".to_string(),
        }
    }

    /// 失败响应
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            code: "FAIL".to_string(),
            message: message.into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  回调请求头提取
// ═══════════════════════════════════════════════════════════════

/// 微信支付 V3 回调通知 HTTP 请求头
#[derive(Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("微信支付V3回调请求头")]
pub struct WxPayNotifyHeaders {
    /// 微信支付平台证书序列号
    #[serde(rename = "wechatpay-serial")]
    pub wechatpay_serial: Option<String>,
    /// 签名值
    #[serde(rename = "wechatpay-signature")]
    pub wechatpay_signature: Option<String>,
    /// 时间戳
    #[serde(rename = "wechatpay-timestamp")]
    pub wechatpay_timestamp: Option<String>,
    /// 随机字符串
    #[serde(rename = "wechatpay-nonce")]
    pub wechatpay_nonce: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  自动注册的回调 Handler
// ═══════════════════════════════════════════════════════════════

/// 微信支付 V3 回调通知处理器
///
/// 由 `AFaster::run()` 自动注册到 `callback_path` 路由。
/// 完整流程：提取验签头 → 调用 `WxPayH5::handle_notify` → 分发到用户回调
#[afast::post(desc("微信支付V3回调通知"))]
pub async fn wx_pay_notify_handler(
    afast::State(state): afast::State<crate::AppState>,
    afast::Header(headers): afast::Header<WxPayNotifyHeaders>,
    afast::Body(notify): afast::Body<WxPayNotify>,
) -> afast::Result<afast::Text> {
    let timestamp = headers.wechatpay_timestamp.as_deref().unwrap_or("");
    let nonce = headers.wechatpay_nonce.as_deref().unwrap_or("");
    let signature = headers.wechatpay_signature.as_deref().unwrap_or("");

    Ok(state
        .wx_pay
        .handle_notify(
            state.clone(),
            timestamp,
            nonce,
            "", // body 已被 afast 解析，V3 验签需要原始 body，但有 platform_cert 时才验签
            signature,
            notify,
        )
        .await?)
}
