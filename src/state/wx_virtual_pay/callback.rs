use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  响应类型
// ═══════════════════════════════════════════════════════════════

/// 消息推送响应
///
/// 微信要求推送响应格式为：
/// - JSON: `{"ErrCode":0,"ErrMsg":"success"}`
/// - XML: `<xml><ErrCode>0</ErrCode><ErrMsg><![CDATA[success]]></ErrMsg></xml>`
///
/// 本框架默认使用 JSON 格式。
#[derive(Debug, Serialize, afast::Tag)]
#[tag("微信虚拟支付推送响应")]
pub struct WxPayNotifyResult {
    #[serde(rename = "ErrCode")]
    pub err_code: i32,
    #[serde(rename = "ErrMsg")]
    pub err_msg: String,
}

impl WxPayNotifyResult {
    /// 成功响应
    pub fn success() -> Self {
        Self {
            err_code: 0,
            err_msg: "success".to_string(),
        }
    }

    /// 失败响应
    pub fn error(err_code: i32, err_msg: impl Into<String>) -> Self {
        Self {
            err_code,
            err_msg: err_msg.into(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  推送请求类型（来自微信服务器）
// ═══════════════════════════════════════════════════════════════

/// 微信支付信息（非微信支付渠道可能没有）
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("微信支付信息")]
#[serde(rename_all = "PascalCase")]
pub struct WeChatPayInfo {
    /// 微信支付商户单号
    pub mch_order_no: Option<String>,
    /// 交易单号（微信支付订单号）
    pub transaction_id: Option<String>,
    /// 用户支付时间，Linux 秒级时间戳
    pub paid_time: Option<i64>,
}

/// 道具参数信息
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("道具参数信息")]
#[serde(rename_all = "PascalCase")]
pub struct GoodsInfo {
    /// 道具 ID
    pub product_id: Option<String>,
    /// 数量
    pub quantity: Option<i32>,
    /// 物品原始价格（单位：分）
    pub orig_price: Option<i64>,
    /// 物品实际支付价格（单位：分）
    pub actual_price: Option<i64>,
    /// 透传信息
    pub attach: Option<String>,
}

/// 代币参数信息
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("代币参数信息")]
#[serde(rename_all = "PascalCase")]
pub struct CoinInfo {
    /// 数量
    pub quantity: Option<i32>,
    /// 物品原始价格（单位：分）
    pub orig_price: Option<i64>,
    /// 物品实际支付价格（单位：分）
    pub actual_price: Option<i64>,
    /// 透传信息
    pub attach: Option<String>,
}

/// 拼团信息
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("拼团信息")]
#[serde(rename_all = "PascalCase")]
pub struct TeamInfo {
    /// 活动 ID
    pub activity_id: Option<String>,
    /// 团 ID
    pub team_id: Option<String>,
    /// 团类型: 1-支付全部，拼成退款
    pub team_type: Option<i32>,
    /// 0-创团 1-参团
    pub team_action: Option<i32>,
}

// ═══════════════════════════════════════════════════════════════
//  四种推送通知请求体
// ═══════════════════════════════════════════════════════════════

/// 道具发货推送请求体（xpay_goods_deliver_notify）
///
/// 用户通过现金购买道具且支付成功后，微信服务器推送此事件。
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("道具发货推送")]
#[serde(rename_all = "PascalCase")]
pub struct WxGoodsDeliverNotify {
    /// 小程序原始 ID
    pub to_user_name: Option<String>,
    /// openid（固定为微信官方的 openid）
    pub from_user_name: Option<String>,
    /// 消息发送时间
    pub create_time: Option<i64>,
    /// 消息类型，固定为 event
    pub msg_type: Option<String>,
    /// 事件类型，固定为 xpay_goods_deliver_notify
    pub event: Option<String>,
    /// 用户 openid
    pub open_id: Option<String>,
    /// 业务订单号
    pub out_trade_no: Option<String>,
    /// 环境配置: 0=现网环境, 1=沙箱环境
    pub env: Option<i32>,
    /// 微信支付信息（非微信支付渠道可能没有）
    pub wechat_pay_info: Option<WeChatPayInfo>,
    /// 道具参数信息
    pub goods_info: Option<GoodsInfo>,
    /// 拼团信息
    pub team_info: Option<TeamInfo>,
    /// 重试次数（从 0 开始）
    pub retry_times: Option<i32>,
}

/// 代币支付推送请求体（xpay_coin_pay_notify）
///
/// 用户代币扣减成功后，微信服务器推送此事件。
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("代币支付推送")]
#[serde(rename_all = "PascalCase")]
pub struct WxCoinPayNotify {
    /// 小程序原始 ID
    pub to_user_name: Option<String>,
    /// openid（固定为微信官方的 openid）
    pub from_user_name: Option<String>,
    /// 消息发送时间
    pub create_time: Option<i64>,
    /// 消息类型，固定为 event
    pub msg_type: Option<String>,
    /// 事件类型，固定为 xpay_coin_pay_notify
    pub event: Option<String>,
    /// 用户 openid
    pub open_id: Option<String>,
    /// 业务订单号
    pub out_trade_no: Option<String>,
    /// 环境配置: 0=现网环境, 1=沙箱环境
    pub env: Option<i32>,
    /// 微信支付信息（非微信支付渠道可能没有）
    pub wechat_pay_info: Option<WeChatPayInfo>,
    /// 代币参数信息
    pub coin_info: Option<CoinInfo>,
    /// 重试次数（从 0 开始）
    pub retry_times: Option<i32>,
}

/// 退款推送请求体（xpay_refund_notify）
///
/// 退款完成后，微信服务器推送此事件。
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("退款推送")]
#[serde(rename_all = "PascalCase")]
pub struct WxRefundNotify {
    /// 小程序原始 ID
    pub to_user_name: Option<String>,
    /// openid（固定为微信官方的 openid）
    pub from_user_name: Option<String>,
    /// 消息发送时间
    pub create_time: Option<i64>,
    /// 消息类型，固定为 event
    pub msg_type: Option<String>,
    /// 事件类型，固定为 xpay_refund_notify
    pub event: Option<String>,
    /// 用户 openid
    pub open_id: Option<String>,
    /// 微信退款单号
    pub wx_refund_id: Option<String>,
    /// 商户退款单号
    pub mch_refund_id: Option<String>,
    /// 退款单对应支付单的微信单号
    pub wx_order_id: Option<String>,
    /// 退款单对应支付单的商户单号
    pub mch_order_id: Option<String>,
    /// 退款金额（单位：分）
    pub refund_fee: Option<i64>,
    /// 退款结果: 0=成功，非 0=失败
    pub ret_code: Option<i32>,
    /// 退款结果详情（失败时为退款失败原因）
    pub ret_msg: Option<String>,
    /// 开始退款时间（秒级时间戳）
    pub refund_start_timestamp: Option<i64>,
    /// 结束退款时间（秒级时间戳）
    pub refund_succ_timestamp: Option<i64>,
    /// 退款单的微信支付单号
    pub wxpay_refund_transaction_id: Option<String>,
    /// 重试次数（从 0 开始）
    pub retry_times: Option<i32>,
    /// 拼团信息
    pub team_info: Option<TeamInfo>,
}

/// 用户投诉推送请求体（xpay_complaint_notify）
///
/// 用户发起投诉后，微信服务器推送此事件。
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("用户投诉推送")]
#[serde(rename_all = "PascalCase")]
pub struct WxComplaintNotify {
    /// 小程序原始 ID
    pub to_user_name: Option<String>,
    /// openid（固定为微信官方的 openid）
    pub from_user_name: Option<String>,
    /// 消息发送时间
    pub create_time: Option<i64>,
    /// 消息类型，固定为 event
    pub msg_type: Option<String>,
    /// 事件类型，固定为 xpay_complaint_notify
    pub event: Option<String>,
    /// 用户 openid
    pub open_id: Option<String>,
    /// 微信单号
    pub wx_order_id: Option<String>,
    /// 商户单号
    pub mch_order_id: Option<String>,
    /// 微信支付交易单号
    pub transaction_id: Option<String>,
    /// 投诉单号
    pub complaint_id: Option<String>,
    /// 投诉详情
    pub complaint_detail: Option<String>,
    /// 投诉时间（秒级时间戳）
    pub complaint_time: Option<i64>,
    /// 重试次数（从 0 开始）
    pub retry_times: Option<i32>,
    /// 请求编号
    pub request_id: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  回调函数类型
// ═══════════════════════════════════════════════════════════════

use crate::state::callbacks::AsyncCallback;

/// 道具发货推送回调函数类型
pub type WxGoodsDeliverCallbackFn =
    AsyncCallback<(crate::AppState, WxGoodsDeliverNotify), crate::Result<WxPayNotifyResult>>;

/// 代币支付推送回调函数类型
pub type WxCoinPayCallbackFn =
    AsyncCallback<(crate::AppState, WxCoinPayNotify), crate::Result<WxPayNotifyResult>>;

/// 退款推送回调函数类型
pub type WxRefundCallbackFn =
    AsyncCallback<(crate::AppState, WxRefundNotify), crate::Result<WxPayNotifyResult>>;

/// 用户投诉推送回调函数类型
pub type WxComplaintCallbackFn =
    AsyncCallback<(crate::AppState, WxComplaintNotify), crate::Result<WxPayNotifyResult>>;

// ═══════════════════════════════════════════════════════════════
//  自动注册的回调 Handler
// ═══════════════════════════════════════════════════════════════

/// 微信虚拟支付 GET 验签处理器
#[afast::get(desc("微信虚拟支付 - 服务器验签"))]
pub async fn verify(
    afast::State(state): afast::State<crate::AppState>,
    afast::Query(query): afast::Query<super::notify::WxVerifyQuery>,
) -> afast::Result<afast::Text> {
    let token = &state.wx_virtual_pay.verify_token;
    if token.is_empty() {
        return Err(super::err::missing_verify_param().into());
    }
    super::notify::verify_signature(token, &query.timestamp, &query.nonce, &query.signature)?;
    Ok(afast::Text(query.echostr))
}

/// 微信虚拟支付 POST 通知分发处理器
#[afast::post(desc("微信虚拟支付 - 通知回调"))]
pub async fn dispatch(
    afast::State(state): afast::State<crate::AppState>,
    afast::Body(body): afast::Body<super::notify::WxNotifyBody>,
) -> afast::Result<afast::Text> {
    Ok(state
        .wx_virtual_pay
        .handle_notify(state.clone(), body)
        .await?)
}
