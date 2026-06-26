pub mod callback;
pub mod err;
pub mod session;
pub use callback::{
    CoinInfo, GoodsInfo, TeamInfo, WeChatPayInfo, WxCoinPayCallbackFn, WxCoinPayNotify,
    WxComplaintCallbackFn, WxComplaintNotify, WxGoodsDeliverCallbackFn, WxGoodsDeliverNotify,
    WxPayNotifyResult, WxRefundCallbackFn, WxRefundNotify,
};

pub use session::{WxMiniLoginResponse, WxSessionManager};

use err::*;

use crate::state::callbacks::IntoCallback;
pub(crate) mod notify;
use notify::WxNotifyBody;

use hmac::{Hmac, KeyInit as _, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn default_callback_path() -> String {
    "wx/virtual-pay/notify".to_string()
}

// ═══════════════════════════════════════════════════════════════
//  配置
// ═══════════════════════════════════════════════════════════════

#[derive(Clone, Deserialize)]
pub struct WxVirtualPay {
    pub offer_id: String,
    pub app_key: String,
    pub env: i32, // 0=正式环境, 1=沙箱环境

    // ── 独立登录凭证（用于获取 session_key）──
    /// 支付小程序的 AppID
    pub mini_id: String,
    /// 支付小程序的 AppSecret
    pub mini_secret: String,

    // ── 回调配置 ──
    /// 回调路由路径，默认 `wx/virtual-pay/notify`
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
    /// 微信消息推送 Token（用于 GET 验签）
    #[serde(default)]
    pub verify_token: String,
    /// 微信消息推送 EncodingAESKey（43 字符，用于 AES 解密）
    #[serde(default)]
    pub msg_secret: String,

    // ── 用户注册的回调函数 ──
    #[serde(skip)]
    pub(crate) goods_deliver_callback: Option<WxGoodsDeliverCallbackFn>,
    #[serde(skip)]
    pub(crate) coin_pay_callback: Option<WxCoinPayCallbackFn>,
    #[serde(skip)]
    pub(crate) refund_callback: Option<WxRefundCallbackFn>,
    #[serde(skip)]
    pub(crate) complaint_callback: Option<WxComplaintCallbackFn>,

    // ── 运行时（serde skip）──
    #[serde(skip)]
    pub client: reqwest::Client,
    /// session_key 管理器，按 `appid:openid` 存储
    #[serde(skip)]
    pub sessions: WxSessionManager,
}

impl WxVirtualPay {
    // ═══════════════════════════════════════════════════════════════
    //  独立登录 (code2Session)
    // ═══════════════════════════════════════════════════════════════

    /// 生成 session 缓存 key: `"appid:openid"`
    pub fn session_key_for(&self, openid: &str) -> String {
        format!("{}:{}", self.mini_id, openid)
    }

    /// 调用微信 jscode2Session 接口登录
    ///
    /// 返回 `WxMiniLoginResponse`，包含 `openid` 和 `session_key`。
    pub async fn mini_login(&self, code: &str) -> crate::Result<WxMiniLoginResponse> {
        let mut url =
            reqwest::Url::parse("https://api.weixin.qq.com/sns/jscode2session").map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50601, msg = "WeChat virtual pay login error" },
                    "{}", _e
                );
                login_url()
            })?;
        url.query_pairs_mut()
            .append_pair("appid", &self.mini_id)
            .append_pair("secret", &self.mini_secret)
            .append_pair("js_code", code)
            .append_pair("grant_type", "authorization_code");

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50602, msg = "WeChat virtual pay login error" },
                    "{}", _e
                );
                login_request()
            })?
            .json::<serde_json::Value>()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50603, msg = "WeChat virtual pay login error" },
                    "{}", _e
                );
                login_response()
            })?;

        // 检查微信 API 错误
        if let Some(errcode) = response.get("errcode").and_then(|v| v.as_i64())
            && errcode != 0
        {
            let errmsg = response
                .get("errmsg")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            #[cfg(feature = "log")]
            tracing::warn!(
                { code = 40601, msg = "WeChat virtual pay login error" },
                "{}", &errmsg
            );
            return Err(login_api(&errmsg));
        }

        let login_response: WxMiniLoginResponse =
            serde_json::from_value(response).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 50604, msg = "WeChat virtual pay login error" },
                    "{}", _e
                );
                login_parse()
            })?;

        // 登录成功，自动缓存 session_key
        let cache_key = self.session_key_for(&login_response.openid);
        self.sessions.set(&cache_key, &login_response.session_key);

        Ok(login_response)
    }

    /// 仅更新 session_key 缓存（当外部已有 session_key 时使用）
    pub fn cache_session(&self, openid: &str, session_key: &str) {
        let cache_key = self.session_key_for(openid);
        self.sessions.set(&cache_key, session_key);
    }

    /// 获取缓存的 session_key
    pub fn get_session(&self, openid: &str) -> Option<String> {
        let cache_key = self.session_key_for(openid);
        self.sessions.get(&cache_key)
    }

    /// 检查微信 API 响应是否包含 session_key 过期错误
    ///
    /// 当微信返回 errcode `-41003`（invalid code / skey expired）时，
    /// 返回 `session_expired()` 错误（40602，"微信登录已过期"）。
    ///
    /// # 用法
    /// ```ignore
    /// let resp: serde_json::Value = client.get(url).send().await?.json().await?;
    /// wx_virtual_pay.check_wx_err(&resp)?;
    /// ```
    pub fn check_wx_err(&self, resp: &serde_json::Value) -> crate::Result<()> {
        if let Some(errcode) = resp.get("errcode").and_then(|v| v.as_i64())
            && errcode == -41003
        {
            #[cfg(feature = "log")]
            tracing::warn!(
                { code = 40602, msg = "WeChat login session expired" },
                "WeChat session_key expired (errcode: -41003)"
            );
            return Err(session_expired());
        }
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════
//  客户端签名 (wx.requestVirtualPayment)
// ═══════════════════════════════════════════════════════════════

/// 代币充值 signData
#[derive(Debug, Serialize)]
pub struct CoinSignData {
    pub offer_id: String,
    pub buy_quantity: i32,
    pub env: i32,
    pub currency_type: String,
    pub out_trade_no: String,
    pub attach: String,
}

/// 道具直购 signData
#[derive(Debug, Serialize)]
pub struct GoodsSignData {
    #[serde(rename = "offerId")]
    pub offer_id: String,
    #[serde(rename = "productId")]
    pub product_id: String,
    #[serde(rename = "buyQuantity")]
    pub buy_quantity: i32,
    pub env: i32,
    #[serde(rename = "currencyType")]
    pub currency_type: String,
    #[serde(rename = "outTradeNo")]
    pub out_trade_no: String,
    #[serde(rename = "goodsPrice")]
    pub goods_price: i64,
    pub attach: String,
}

impl WxVirtualPay {
    /// 生成代币充值 signData JSON
    pub fn coin_sign_data(&self, order_no: &str, quantity: i32, attach: &str) -> String {
        let data = serde_json::json!({
            "offerId": self.offer_id,
            "buyQuantity": quantity,
            "env": self.env,
            "currencyType": "CNY",
            "outTradeNo": order_no,
            "attach": attach,
        });
        serde_json::to_string(&data).unwrap()
    }

    /// 生成道具直购 signData JSON
    pub fn goods_sign_data(
        &self,
        order_no: &str,
        product_id: &str,
        quantity: i32,
        goods_price: i64,
        attach: &str,
    ) -> String {
        let data = serde_json::json!({
            "offerId": self.offer_id,
            "productId": product_id,
            "buyQuantity": quantity,
            "env": self.env,
            "currencyType": "CNY",
            "outTradeNo": order_no,
            "goodsPrice": goods_price,
            "attach": attach,
        });
        serde_json::to_string(&data).unwrap()
    }

    /// 计算客户端 paySig (URI = "requestVirtualPayment")
    ///
    /// paySig = to_hex(hmac_sha256(appKey, "requestVirtualPayment&" + signData))
    pub fn client_pay_sig(&self, sign_data_json: &str) -> String {
        self.calc_pay_sig("requestVirtualPayment", sign_data_json)
    }

    /// 计算客户端 signature
    ///
    /// signature = to_hex(hmac_sha256(sessionKey, signData))
    pub fn client_signature(session_key: &str, sign_data_json: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(session_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(sign_data_json.as_bytes());
        to_hex(&mac.finalize().into_bytes())
    }

    // ═══════════════════════════════════════════════════════════════
    //  服务器端 API 签名
    // ═══════════════════════════════════════════════════════════════

    /// 计算服务器端 paySig
    ///
    /// paySig = to_hex(hmac_sha256(appKey, uri + "&" + post_body))
    pub fn calc_pay_sig(&self, uri: &str, post_body: &str) -> String {
        let message = format!("{}&{}", uri, post_body);
        let mut mac = HmacSha256::new_from_slice(self.app_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        to_hex(&mac.finalize().into_bytes())
    }

    /// 计算服务器端 signature
    ///
    /// signature = to_hex(hmac_sha256(sessionKey, post_body))
    pub fn calc_signature(session_key: &str, post_body: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(session_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(post_body.as_bytes());
        to_hex(&mac.finalize().into_bytes())
    }

    // ═══════════════════════════════════════════════════════════════
    //  消息推送签名验证
    // ═══════════════════════════════════════════════════════════════

    /// 验证消息推送签名
    ///
    /// 签名算法: to_hex(hmac_sha256(appKey, event + "&" + payload))
    pub fn verify_push_signature(&self, event: &str, payload: &str, sig: &str) -> bool {
        let message = format!("{}&{}", event, payload);
        let mut mac = HmacSha256::new_from_slice(self.app_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        to_hex(&mac.finalize().into_bytes()) == sig
    }

    // ═══════════════════════════════════════════════════════════════
    //  代币相关 API 请求体构建
    // ═══════════════════════════════════════════════════════════════

    /// 查询代币余额请求体
    pub fn query_user_balance_body(openid: &str, user_ip: &str, env: i32) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
        })
        .to_string()
    }

    /// 扣减代币请求体
    pub fn currency_pay_body(
        openid: &str,
        user_ip: &str,
        env: i32,
        out_trade_no: &str,
        quantity: i32,
        attach: &str,
    ) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
            "out_trade_no": out_trade_no,
            "quantity": quantity,
            "attach": attach,
        })
        .to_string()
    }

    /// 代币支付退款请求体
    pub fn cancel_currency_pay_body(
        openid: &str,
        user_ip: &str,
        env: i32,
        out_trade_no: &str,
    ) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
            "out_trade_no": out_trade_no,
        })
        .to_string()
    }

    /// 代币赠送请求体
    pub fn present_currency_body(
        openid: &str,
        user_ip: &str,
        env: i32,
        out_trade_no: &str,
        quantity: i32,
        attach: &str,
    ) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
            "out_trade_no": out_trade_no,
            "quantity": quantity,
            "attach": attach,
        })
        .to_string()
    }

    // ═══════════════════════════════════════════════════════════════
    //  订单与账单 API 请求体构建
    // ═══════════════════════════════════════════════════════════════

    /// 查询订单请求体
    pub fn query_order_body(openid: &str, user_ip: &str, env: i32, out_trade_no: &str) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
            "out_trade_no": out_trade_no,
        })
        .to_string()
    }

    /// 通知已发货完成请求体
    pub fn notify_provide_goods_body(
        openid: &str,
        user_ip: &str,
        env: i32,
        out_trade_no: &str,
    ) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
            "out_trade_no": out_trade_no,
        })
        .to_string()
    }

    /// 启动订单退款任务请求体
    pub fn refund_order_body(
        openid: &str,
        user_ip: &str,
        env: i32,
        out_trade_no: &str,
        refund_out_trade_no: &str,
        refund_fee: i64,
    ) -> String {
        serde_json::json!({
            "openid": openid,
            "user_ip": user_ip,
            "env": env,
            "out_trade_no": out_trade_no,
            "refund_out_trade_no": refund_out_trade_no,
            "refund_fee": refund_fee,
        })
        .to_string()
    }

    // ═══════════════════════════════════════════════════════════════
    //  回调注册
    // ═══════════════════════════════════════════════════════════════

    /// 设置道具发货回调
    pub fn with_goods_deliver_callback(
        mut self,
        cb: impl IntoCallback<(crate::AppState, WxGoodsDeliverNotify), crate::Result<WxPayNotifyResult>>,
    ) -> Self {
        self.goods_deliver_callback = Some(cb.into_callback());
        self
    }

    /// 设置代币支付回调
    pub fn with_coin_pay_callback(
        mut self,
        cb: impl IntoCallback<(crate::AppState, WxCoinPayNotify), crate::Result<WxPayNotifyResult>>,
    ) -> Self {
        self.coin_pay_callback = Some(cb.into_callback());
        self
    }

    /// 设置退款回调
    pub fn with_refund_callback(
        mut self,
        cb: impl IntoCallback<(crate::AppState, WxRefundNotify), crate::Result<WxPayNotifyResult>>,
    ) -> Self {
        self.refund_callback = Some(cb.into_callback());
        self
    }

    /// 设置用户投诉回调
    pub fn with_complaint_callback(
        mut self,
        cb: impl IntoCallback<(crate::AppState, WxComplaintNotify), crate::Result<WxPayNotifyResult>>,
    ) -> Self {
        self.complaint_callback = Some(cb.into_callback());
        self
    }

    // ═══════════════════════════════════════════════════════════════
    //  回调分发（供 afast handler 调用）
    // ═══════════════════════════════════════════════════════════════

    /// 处理微信虚拟支付回调通知
    ///
    /// 完整流程：解密 → 按 Event 分发到用户回调
    pub async fn handle_notify(
        &self,
        state: crate::AppState,
        mut body: WxNotifyBody,
    ) -> crate::Result<afast::Text> {
        // 如果有 Encrypt 字段，解密
        if let Some(encrypt_b64) = &body.encrypt {
            if self.msg_secret.is_empty() {
                #[cfg(feature = "log")]
                tracing::error!("WxVirtualPay: encrypted message but msg_secret not configured");
                return Err(err::decrypt_failed("msg_secret not configured"));
            }
            let json_str = notify::decrypt_wx_message(encrypt_b64, &self.msg_secret)?;
            body = serde_json::from_str(&json_str).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!("WxVirtualPay decrypted message parse failed: {}", _e);
                crate::Error::custom(50801, "Decrypted message parse failed")
            })?;
        }

        let body_json = serde_json::to_value(&body).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!("WxVirtualPay notification serialize error: {}", _e);
            crate::Error::custom(50801, "Notification body serialize failed")
        })?;

        match body.event.as_deref() {
            Some("xpay_goods_deliver_notify") => {
                let notify: WxGoodsDeliverNotify =
                    serde_json::from_value(body_json).map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!("WxVirtualPay goods deliver parse failed: {}", _e);
                        crate::Error::custom(50802, "Goods delivery notification parse failed")
                    })?;
                if let Some(cb) = self.goods_deliver_callback.clone() {
                    let result = cb((state.clone(), notify)).await?;
                    Ok(afast::Text(
                        serde_json::to_string(&result).unwrap_or_default(),
                    ))
                } else {
                    #[cfg(feature = "log")]
                    tracing::debug!("WxVirtualPay goods_deliver callback not registered");
                    Ok(afast::Text(
                        r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                    ))
                }
            }
            Some("xpay_coin_pay_notify") => {
                let notify: WxCoinPayNotify = serde_json::from_value(body_json).map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!("WxVirtualPay coin pay parse failed: {}", _e);
                    crate::Error::custom(50803, "Coin payment notification parse failed")
                })?;
                if let Some(cb) = self.coin_pay_callback.clone() {
                    let result = cb((state.clone(), notify)).await?;
                    Ok(afast::Text(
                        serde_json::to_string(&result).unwrap_or_default(),
                    ))
                } else {
                    #[cfg(feature = "log")]
                    tracing::debug!("WxVirtualPay coin_pay callback not registered");
                    Ok(afast::Text(
                        r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                    ))
                }
            }
            Some("xpay_refund_notify") => {
                let notify: WxRefundNotify = serde_json::from_value(body_json).map_err(|_e| {
                    #[cfg(feature = "log")]
                    tracing::error!("WxVirtualPay refund parse failed: {}", _e);
                    crate::Error::custom(50804, "Refund notification parse failed")
                })?;
                if let Some(cb) = self.refund_callback.clone() {
                    let result = cb((state.clone(), notify)).await?;
                    Ok(afast::Text(
                        serde_json::to_string(&result).unwrap_or_default(),
                    ))
                } else {
                    #[cfg(feature = "log")]
                    tracing::debug!("WxVirtualPay refund callback not registered");
                    Ok(afast::Text(
                        r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                    ))
                }
            }
            Some("xpay_complaint_notify") => {
                let notify: WxComplaintNotify =
                    serde_json::from_value(body_json).map_err(|_e| {
                        #[cfg(feature = "log")]
                        tracing::error!("WxVirtualPay complaint parse failed: {}", _e);
                        crate::Error::custom(50805, "Complaint notification parse failed")
                    })?;
                if let Some(cb) = self.complaint_callback.clone() {
                    let result = cb((state.clone(), notify)).await?;
                    Ok(afast::Text(
                        serde_json::to_string(&result).unwrap_or_default(),
                    ))
                } else {
                    #[cfg(feature = "log")]
                    tracing::debug!("WxVirtualPay complaint callback not registered");
                    Ok(afast::Text(
                        r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                    ))
                }
            }
            _ => {
                #[cfg(feature = "log")]
                tracing::warn!("WxVirtualPay unknown event: {:?}", body.event);
                Ok(afast::Text(
                    r#"{"ErrCode":0,"ErrMsg":"success"}"#.to_string(),
                ))
            }
        }
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "wx_virtual_pay")?;
        instance.client = reqwest::Client::new();
        Ok(instance)
    }
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 微信虚拟支付配置扩展
pub trait AFasterWxVirtualPayExt {
    /// 链式配置微信虚拟支付
    fn with_wx_virtual_pay(self, f: impl FnOnce(WxVirtualPay) -> WxVirtualPay) -> Self;
}

impl AFasterWxVirtualPayExt for crate::AFaster {
    fn with_wx_virtual_pay(mut self, f: impl FnOnce(WxVirtualPay) -> WxVirtualPay) -> Self {
        self.state.wx_virtual_pay = f(self.state.wx_virtual_pay);
        self
    }
}
