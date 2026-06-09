//! 微信支付 V3 公共模块
//!
//! 提供 H5 / Native / APP 三种支付方式共用的：
//! - V3 API 签名
//! - 查询订单、关闭订单
//! - 退款（申请、查询、异常退款）
//! - 账单（交易账单、资金账单、下载）
//! - 回调通知（解密、验签、分发）

pub mod callback;
pub mod err;

pub use callback::{
    WxPayAmount, WxPayNotify, WxPayNotifyResource, WxPayNotifyResult, WxPayPayer,
    WxPayRefundNotify, WxPayTransactionNotify,
};

use err::*;

use crate::state::callbacks::AsyncCallback;

use base64::Engine as _;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  回调函数类型
// ═══════════════════════════════════════════════════════════════

/// 支付成功回调函数类型
pub type PaySuccessCallbackFn =
    AsyncCallback<(crate::AppState, WxPayTransactionNotify), crate::Result<WxPayNotifyResult>>;

/// 退款回调函数类型
pub type RefundCallbackFn =
    AsyncCallback<(crate::AppState, WxPayRefundNotify), crate::Result<WxPayNotifyResult>>;

// ═══════════════════════════════════════════════════════════════
//  公共配置字段（供各支付方式嵌套使用）
// ═══════════════════════════════════════════════════════════════

/// 微信支付 V3 公共配置
///
/// 各支付方式（H5/Native/APP）的 config 中嵌套此结构。
#[derive(Clone, Deserialize)]
pub struct WxPayConfig {
    /// 商户号
    pub mch_id: String,
    /// 应用 ID (AppID)
    pub app_id: String,
    /// API v3 密钥 (用于 AEAD_AES_256_GCM 解密回调通知)
    pub api_v3_key: String,
    /// 商户 API 私钥 (PEM 格式字符串)
    pub private_key: String,
    /// 商户证书序列号
    pub serial_no: String,
    /// 支付结果通知回调完整 URL（发给微信）
    #[serde(default)]
    pub notify_url: Option<String>,
    /// 退款结果通知回调完整 URL（可选，默认使用 notify_url）
    #[serde(default)]
    pub refund_notify_url: Option<String>,
    /// 微信支付平台证书公钥 (PEM 格式，用于验签回调通知)
    #[serde(default)]
    pub platform_cert: Option<String>,
}

/// 运行时状态（不序列化）
#[derive(Clone, Default)]
pub struct WxPayRuntime {
    pub(crate) client: reqwest::Client,
    pub(crate) private_key_parsed: Option<rsa::RsaPrivateKey>,
    pub(crate) pay_success_callback: Option<PaySuccessCallbackFn>,
    pub(crate) refund_callback: Option<RefundCallbackFn>,
}

/// 微信支付 V3 统一模块
///
/// 支持 H5 / Native / APP 三种支付方式，通过 feature 控制。
#[derive(Clone, Deserialize)]
pub struct WxPay {
    #[serde(flatten)]
    pub config: WxPayConfig,
    /// 回调通知路由路径
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
    #[serde(skip)]
    pub(crate) runtime: WxPayRuntime,
}

fn default_callback_path() -> String {
    "wx/pay/notify".to_string()
}

impl WxPay {
    /// 初始化运行时（加载私钥、创建 HTTP 客户端）
    pub fn init(&mut self) -> crate::Result<()> {
        self.runtime.client = reqwest::Client::new();
        self.runtime.private_key_parsed = Some(load_private_key(&self.config.private_key)?);
        Ok(())
    }

    /// 注册支付成功回调
    pub fn with_pay_success_callback(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<
            (crate::AppState, WxPayTransactionNotify),
            crate::Result<WxPayNotifyResult>,
        >,
    ) -> Self {
        self.runtime.pay_success_callback = Some(cb.into_callback());
        self
    }

    /// 注册退款回调
    pub fn with_refund_callback(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<
            (crate::AppState, WxPayRefundNotify),
            crate::Result<WxPayNotifyResult>,
        >,
    ) -> Self {
        self.runtime.refund_callback = Some(cb.into_callback());
        self
    }

    // ── 公共方法（查询、退款、账单等）──

    pub async fn query_by_out_trade_no(
        &self,
        out_trade_no: &str,
    ) -> crate::Result<WxPayTransactionNotify> {
        query_by_out_trade_no(&self.runtime.client, &self.config, out_trade_no).await
    }

    pub async fn query_by_transaction_id(
        &self,
        transaction_id: &str,
    ) -> crate::Result<WxPayTransactionNotify> {
        query_by_transaction_id(&self.runtime.client, &self.config, transaction_id).await
    }

    pub async fn close_order(&self, out_trade_no: &str) -> crate::Result<()> {
        close_order(&self.runtime.client, &self.config, out_trade_no).await
    }

    pub async fn refund(
        &self,
        out_refund_no: &str,
        out_trade_no: &str,
        amount: i64,
        total: i64,
        reason: Option<&str>,
    ) -> crate::Result<RefundResult> {
        refund(
            &self.runtime.client,
            &self.config,
            out_refund_no,
            out_trade_no,
            amount,
            total,
            reason,
        )
        .await
    }

    pub async fn query_refund(&self, out_refund_no: &str) -> crate::Result<RefundResult> {
        query_refund(&self.runtime.client, &self.config, out_refund_no).await
    }

    pub async fn apply_abnormal_refund(
        &self,
        refund_id: &str,
        out_refund_no: &str,
        abnormal_type: &str,
    ) -> crate::Result<RefundResult> {
        apply_abnormal_refund(
            &self.runtime.client,
            &self.config,
            refund_id,
            out_refund_no,
            abnormal_type,
        )
        .await
    }

    pub async fn apply_trade_bill(
        &self,
        bill_date: &str,
        bill_type: Option<&str>,
    ) -> crate::Result<BillDownloadResult> {
        apply_trade_bill(&self.runtime.client, &self.config, bill_date, bill_type).await
    }

    pub async fn apply_fund_flow_bill(
        &self,
        bill_date: &str,
        account_type: Option<&str>,
    ) -> crate::Result<BillDownloadResult> {
        apply_fund_flow_bill(&self.runtime.client, &self.config, bill_date, account_type).await
    }

    pub async fn download_bill(&self, download_url: &str) -> crate::Result<String> {
        download_bill(&self.runtime.client, &self.config, download_url).await
    }

    pub fn decrypt_transaction_notify(
        &self,
        resource: &WxPayNotifyResource,
    ) -> crate::Result<WxPayTransactionNotify> {
        decrypt_transaction_notify(&self.config.api_v3_key, resource)
    }

    pub fn decrypt_refund_notify(
        &self,
        resource: &WxPayNotifyResource,
    ) -> crate::Result<WxPayRefundNotify> {
        decrypt_refund_notify(&self.config.api_v3_key, resource)
    }

    /// 处理回调通知（验签 → 解密 → 分发到用户回调）
    pub async fn handle_notify(
        &self,
        state: crate::AppState,
        timestamp: &str,
        nonce: &str,
        body: &str,
        signature: &str,
        notify: WxPayNotify,
    ) -> crate::Result<afast::Text> {
        dispatch_notify(
            &self.config.api_v3_key,
            self.config.platform_cert.as_deref(),
            self.runtime.pay_success_callback.as_ref(),
            self.runtime.refund_callback.as_ref(),
            state,
            timestamp,
            nonce,
            body,
            signature,
            notify.resource,
            &notify.event_type,
        )
        .await
    }

    // ── H5 预下单 ──
    #[cfg(feature = "wx-pay-h5")]
    pub async fn prepay_h5(
        &self,
        out_trade_no: &str,
        description: &str,
        amount_total: i64,
        payer_client_ip: &str,
        h5_info_type: &str,
        scene_url: &str,
    ) -> crate::Result<H5PrepayResult> {
        prepay_h5(
            &self.runtime.client,
            &self.config,
            out_trade_no,
            description,
            amount_total,
            payer_client_ip,
            h5_info_type,
            scene_url,
        )
        .await
    }

    // ── Native 预下单 ──
    #[cfg(feature = "wx-pay-native")]
    pub async fn prepay_native(
        &self,
        out_trade_no: &str,
        description: &str,
        amount_total: i64,
    ) -> crate::Result<NativePrepayResult> {
        prepay_native(
            &self.runtime.client,
            &self.config,
            out_trade_no,
            description,
            amount_total,
        )
        .await
    }

    // ── APP 预下单 ──
    #[cfg(feature = "wx-pay-app")]
    pub async fn prepay_app(
        &self,
        out_trade_no: &str,
        description: &str,
        amount_total: i64,
    ) -> crate::Result<AppPrepayResult> {
        prepay_app(
            &self.runtime.client,
            &self.config,
            out_trade_no,
            description,
            amount_total,
        )
        .await
    }

    // ── 小程序 预下单 ──
    #[cfg(feature = "wx-pay-mini")]
    pub async fn prepay_mini(
        &self,
        out_trade_no: &str,
        description: &str,
        amount_total: i64,
        openid: &str,
    ) -> crate::Result<MiniPrepayResult> {
        prepay_js_mini(
            &self.runtime.client,
            &self.config,
            out_trade_no,
            description,
            amount_total,
            openid,
        )
        .await
    }

    // ── JSAPI 预下单 ──
    #[cfg(feature = "wx-pay-js")]
    pub async fn prepay_js(
        &self,
        out_trade_no: &str,
        description: &str,
        amount_total: i64,
        openid: &str,
    ) -> crate::Result<MiniPrepayResult> {
        prepay_js_mini(
            &self.runtime.client,
            &self.config,
            out_trade_no,
            description,
            amount_total,
            openid,
        )
        .await
    }
}

// ═══════════════════════════════════════════════════════════════
//  V3 API 签名
// ═══════════════════════════════════════════════════════════════

pub fn build_authorization(
    mch_id: &str,
    serial_no: &str,
    private_key: &rsa::RsaPrivateKey,
    method: &str,
    url_path: &str,
    body: &str,
) -> crate::Result<String> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let nonce_str = generate_nonce(32);
    let message = format!(
        "{}\n{}\n{}\n{}\n{}\n",
        method, url_path, timestamp, nonce_str, body
    );
    use rsa::pkcs1v15::SigningKey;
    use rsa::signature::{SignatureEncoding, Signer};
    let signing_key = SigningKey::<sha2_v10::Sha256>::new(private_key.clone());
    let signature = signing_key
        .try_sign(message.as_bytes())
        .map_err(|_| sign_failed())?;
    let signature_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    Ok(format!(
        r#"WECHATPAY2-SHA256-RSA2048 mchid="{}",nonce_str="{}",timestamp="{}",serial_no="{}",signature="{}""#,
        mch_id, nonce_str, timestamp, serial_no, signature_b64
    ))
}

/// 通用签名请求发送
pub async fn send_v3_request(
    client: &reqwest::Client,
    mch_id: &str,
    serial_no: &str,
    private_key: &rsa::RsaPrivateKey,
    method: &str,
    url_path: &str,
    body: &str,
) -> crate::Result<reqwest::Response> {
    let url = format!("https://api.mch.weixin.qq.com{}", url_path);
    let authorization =
        build_authorization(mch_id, serial_no, private_key, method, url_path, body)?;
    let mut req = match method {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        _ => client.get(&url),
    };
    req = req
        .header("Authorization", &authorization)
        .header("Accept", "application/json");
    if method == "POST" {
        req = req
            .header("Content-Type", "application/json")
            .body(body.to_string());
    }
    req.send().await.map_err(|_e| {
        #[cfg(feature = "log")]
        tracing::error!("WxPay request failed: {}", _e);
        request_failed()
    })
}

/// 处理非 2xx 响应
pub async fn handle_error_response(resp: reqwest::Response, _context: &str) -> crate::Error {
    let err_body: serde_json::Value = resp.json().await.unwrap_or_default();
    let code = err_body
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    let message = err_body
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error");
    #[cfg(feature = "log")]
    tracing::error!("WxPay {} error: {} - {}", context, code, message);
    api_error(code, message)
}

// ═══════════════════════════════════════════════════════════════
//  查询订单
// ═══════════════════════════════════════════════════════════════

pub async fn query_by_out_trade_no(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_trade_no: &str,
) -> crate::Result<WxPayTransactionNotify> {
    let private_key = get_private_key(config)?;
    let url_path = format!(
        "/v3/pay/transactions/out-trade-no/{}?mchid={}",
        out_trade_no, config.mch_id
    );
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "GET",
        &url_path,
        "",
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "query_order").await);
    }
    resp.json().await.map_err(|_e| query_response())
}

pub async fn query_by_transaction_id(
    client: &reqwest::Client,
    config: &WxPayConfig,
    transaction_id: &str,
) -> crate::Result<WxPayTransactionNotify> {
    let private_key = get_private_key(config)?;
    let url_path = format!(
        "/v3/pay/transactions/id/{}?mchid={}",
        transaction_id, config.mch_id
    );
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "GET",
        &url_path,
        "",
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "query_order").await);
    }
    resp.json().await.map_err(|_e| query_response())
}

// ═══════════════════════════════════════════════════════════════
//  关闭订单
// ═══════════════════════════════════════════════════════════════

pub async fn close_order(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_trade_no: &str,
) -> crate::Result<()> {
    let private_key = get_private_key(config)?;
    let url_path = format!("/v3/pay/transactions/out-trade-no/{}/close", out_trade_no);
    let body = serde_json::json!({"mchid": config.mch_id});
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        &url_path,
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "close_order").await);
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
//  退款
// ═══════════════════════════════════════════════════════════════

pub async fn refund(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_refund_no: &str,
    out_trade_no: &str,
    refund_amount: i64,
    total: i64,
    reason: Option<&str>,
) -> crate::Result<RefundResult> {
    let private_key = get_private_key(config)?;
    let mut body = serde_json::json!({
        "out_refund_no": out_refund_no,
        "out_trade_no": out_trade_no,
        "amount": { "refund": refund_amount, "total": total, "currency": "CNY" }
    });
    if let Some(r) = reason {
        body["reason"] = serde_json::Value::String(r.to_string());
    }
    if let Some(url) = &config.refund_notify_url {
        body["notify_url"] = serde_json::Value::String(url.clone());
    }
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        "/v3/refund/domestic/refunds",
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "refund").await);
    }
    resp.json().await.map_err(|_e| refund_response())
}

pub async fn query_refund(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_refund_no: &str,
) -> crate::Result<RefundResult> {
    let private_key = get_private_key(config)?;
    let url_path = format!("/v3/refund/domestic/refunds/{}", out_refund_no);
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "GET",
        &url_path,
        "",
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "query_refund").await);
    }
    resp.json().await.map_err(|_e| refund_response())
}

pub async fn apply_abnormal_refund(
    client: &reqwest::Client,
    config: &WxPayConfig,
    refund_id: &str,
    out_refund_no: &str,
    abnormal_type: &str,
) -> crate::Result<RefundResult> {
    let private_key = get_private_key(config)?;
    let url_path = format!(
        "/v3/refund/domestic/refunds/{}/apply-abnormal-refund",
        refund_id
    );
    let body = serde_json::json!({"out_refund_no": out_refund_no, "type": abnormal_type});
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        &url_path,
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "apply_abnormal_refund").await);
    }
    resp.json().await.map_err(|_e| refund_response())
}

// ═══════════════════════════════════════════════════════════════
//  账单
// ═══════════════════════════════════════════════════════════════

pub async fn apply_trade_bill(
    client: &reqwest::Client,
    config: &WxPayConfig,
    bill_date: &str,
    bill_type: Option<&str>,
) -> crate::Result<BillDownloadResult> {
    let private_key = get_private_key(config)?;
    let mut url_path = format!("/v3/bill/tradebill?bill_date={}", bill_date);
    if let Some(t) = bill_type {
        url_path.push_str(&format!("&bill_type={}", t));
    }
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "GET",
        &url_path,
        "",
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "apply_trade_bill").await);
    }
    resp.json().await.map_err(|_e| bill_response())
}

pub async fn apply_fund_flow_bill(
    client: &reqwest::Client,
    config: &WxPayConfig,
    bill_date: &str,
    account_type: Option<&str>,
) -> crate::Result<BillDownloadResult> {
    let private_key = get_private_key(config)?;
    let mut url_path = format!("/v3/bill/fundflowbill?bill_date={}", bill_date);
    if let Some(t) = account_type {
        url_path.push_str(&format!("&account_type={}", t));
    }
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "GET",
        &url_path,
        "",
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "apply_fund_flow_bill").await);
    }
    resp.json().await.map_err(|_e| bill_response())
}

pub async fn download_bill(
    client: &reqwest::Client,
    config: &WxPayConfig,
    download_url: &str,
) -> crate::Result<String> {
    let private_key = get_private_key(config)?;
    let authorization = build_authorization(
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "GET",
        download_url,
        "",
    )?;
    let resp = client
        .get(download_url)
        .header("Authorization", &authorization)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_e| request_failed())?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "download_bill").await);
    }
    resp.text().await.map_err(|_e| bill_response())
}

// ═══════════════════════════════════════════════════════════════
//  回调解密
// ═══════════════════════════════════════════════════════════════

pub fn decrypt_notification(
    api_v3_key: &str,
    resource: &WxPayNotifyResource,
) -> crate::Result<String> {
    let api_key = api_v3_key.as_bytes();
    if api_key.len() != 32 {
        return Err(notify_decrypt("api_v3_key must be 32 bytes"));
    }
    let nonce = resource.nonce.as_bytes();
    let associated_data = resource.associated_data.as_deref().unwrap_or("").as_bytes();
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(&resource.ciphertext)
        .map_err(|e| notify_decrypt(&format!("base64 decode failed: {}", e)))?;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
    let cipher = Aes256Gcm::new_from_slice(api_key)
        .map_err(|e| notify_decrypt(&format!("AES-GCM init failed: {}", e)))?;
    let nonce = Nonce::from_slice(nonce);
    let plaintext = cipher
        .decrypt(
            nonce,
            aes_gcm::aead::Payload {
                msg: &ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|e| notify_decrypt(&format!("decrypt failed: {}", e)))?;
    String::from_utf8(plaintext).map_err(|e| notify_decrypt(&format!("utf8 decode failed: {}", e)))
}

pub fn decrypt_transaction_notify(
    api_v3_key: &str,
    notify: &WxPayNotifyResource,
) -> crate::Result<WxPayTransactionNotify> {
    let json_str = decrypt_notification(api_v3_key, notify)?;
    serde_json::from_str(&json_str)
        .map_err(|_e| crate::Error::custom(50911, "WxPay transaction notify parse failed"))
}

pub fn decrypt_refund_notify(
    api_v3_key: &str,
    notify: &WxPayNotifyResource,
) -> crate::Result<WxPayRefundNotify> {
    let json_str = decrypt_notification(api_v3_key, notify)?;
    serde_json::from_str(&json_str)
        .map_err(|_e| crate::Error::custom(50912, "WxPay refund notify parse failed"))
}

// ═══════════════════════════════════════════════════════════════
//  回调验签
// ═══════════════════════════════════════════════════════════════

pub fn verify_notification_signature(
    timestamp: &str,
    nonce: &str,
    body: &str,
    signature: &str,
    cert_pem: &str,
) -> crate::Result<()> {
    let message = format!("{}\n{}\n{}\n", timestamp, nonce, body);
    let cert = load_certificate_public_key(cert_pem)?;
    use rsa::pkcs1v15::VerifyingKey;
    use rsa::signature::Verifier;
    let verifying_key = VerifyingKey::<sha2_v10::Sha256>::new(cert);
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature)
        .map_err(|_| notify_verify())?;
    let signature_obj =
        rsa::pkcs1v15::Signature::try_from(sig_bytes.as_slice()).map_err(|_| notify_verify())?;
    verifying_key
        .verify(message.as_bytes(), &signature_obj)
        .map_err(|_| notify_verify())
}

// ═══════════════════════════════════════════════════════════════
//  回调分发（通用，供各支付方式的 handle_notify 调用）
// ═══════════════════════════════════════════════════════════════

#[allow(clippy::too_many_arguments)]
pub async fn dispatch_notify(
    api_v3_key: &str,
    platform_cert: Option<&str>,
    pay_success_callback: Option<&PaySuccessCallbackFn>,
    refund_callback: Option<&RefundCallbackFn>,
    state: crate::AppState,
    timestamp: &str,
    nonce: &str,
    body: &str,
    signature: &str,
    notify: WxPayNotifyResource,
    event_type: &str,
) -> crate::Result<afast::Text> {
    // 验签
    if let Some(cert_pem) = platform_cert
        && let Err(_e) = verify_notification_signature(timestamp, nonce, body, signature, cert_pem)
    {
        #[cfg(feature = "log")]
        tracing::error!("WxPay notify verify failed: {}", _e);
        return Ok(afast::Text(
            serde_json::to_string(&WxPayNotifyResult::fail("verify failed")).unwrap_or_default(),
        ));
    }

    match event_type {
        "TRANSACTION.SUCCESS" => match decrypt_transaction_notify(api_v3_key, &notify) {
            Ok(tx_notify) => {
                if let Some(cb) = pay_success_callback {
                    let resp = cb((state, tx_notify)).await?;
                    Ok(afast::Text(
                        serde_json::to_string(&resp).unwrap_or_default(),
                    ))
                } else {
                    Ok(afast::Text(
                        serde_json::to_string(&WxPayNotifyResult::success()).unwrap_or_default(),
                    ))
                }
            }
            Err(_e) => Ok(afast::Text(
                serde_json::to_string(&WxPayNotifyResult::fail("decrypt failed"))
                    .unwrap_or_default(),
            )),
        },
        "REFUND.SUCCESS" | "REFUND.ABNORMAL" | "REFUND.CLOSED" => {
            match decrypt_refund_notify(api_v3_key, &notify) {
                Ok(refund_notify) => {
                    if let Some(cb) = refund_callback {
                        let resp = cb((state, refund_notify)).await?;
                        Ok(afast::Text(
                            serde_json::to_string(&resp).unwrap_or_default(),
                        ))
                    } else {
                        Ok(afast::Text(
                            serde_json::to_string(&WxPayNotifyResult::success())
                                .unwrap_or_default(),
                        ))
                    }
                }
                Err(_e) => Ok(afast::Text(
                    serde_json::to_string(&WxPayNotifyResult::fail("decrypt failed"))
                        .unwrap_or_default(),
                )),
            }
        }
        _ => Ok(afast::Text(
            serde_json::to_string(&WxPayNotifyResult::success()).unwrap_or_default(),
        )),
    }
}

// ═══════════════════════════════════════════════════════════════
//  返回类型
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RefundResult {
    pub refund_id: String,
    pub out_refund_no: String,
    pub transaction_id: Option<String>,
    pub out_trade_no: String,
    pub channel: Option<String>,
    pub user_received_account: Option<String>,
    pub success_time: Option<String>,
    pub create_time: Option<String>,
    pub status: String,
    pub funds_account: Option<String>,
    pub amount: Option<RefundAmount>,
    pub promotion_detail: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RefundAmount {
    pub total: i64,
    pub refund: i64,
    pub payer_total: Option<i64>,
    pub payer_refund: Option<i64>,
    pub settlement_refund: Option<i64>,
    pub settlement_total: Option<i64>,
    pub discount_refund: Option<i64>,
    pub currency: Option<String>,
    pub refund_fee: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("账单下载响应")]
pub struct BillDownloadResult {
    pub hash_type: String,
    pub hash_value: String,
    pub download_url: String,
}

// ═══════════════════════════════════════════════════════════════
//  工具函数
// ═══════════════════════════════════════════════════════════════

pub fn get_private_key(config: &WxPayConfig) -> crate::Result<rsa::RsaPrivateKey> {
    load_private_key(&config.private_key)
}

pub fn load_private_key(pem_str: &str) -> crate::Result<rsa::RsaPrivateKey> {
    use rsa::pkcs8::DecodePrivateKey;
    let pem = pem::parse(pem_str).map_err(|_e| private_key_load())?;
    rsa::RsaPrivateKey::from_pkcs8_der(pem.contents()).map_err(|_e| private_key_load())
}

fn load_certificate_public_key(pem_str: &str) -> crate::Result<rsa::RsaPublicKey> {
    use rsa::pkcs8::DecodePublicKey;
    use x509_certificate::X509Certificate;
    let cert = X509Certificate::from_pem(pem_str)
        .map_err(|_e| crate::Error::custom(50913, "WxPay certificate parse failed"))?;
    let public_key_der = cert.public_key_data();
    rsa::RsaPublicKey::from_public_key_der(&public_key_der)
        .map_err(|_e| crate::Error::custom(50913, "WxPay certificate public key parse failed"))
}

pub fn generate_nonce(len: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..len)
        .map(|_| chars[rng.gen_range(0..chars.len())] as char)
        .collect()
}

// ═══════════════════════════════════════════════════════════════
//  预下单（按 feature 条件编译）
// ═══════════════════════════════════════════════════════════════

/// H5 预下单响应
#[cfg(feature = "wx-pay-h5")]
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("H5预下单响应")]
pub struct H5PrepayResult {
    /// 支付跳转链接（前端重定向到此 URL 拉起微信支付）
    pub h5_url: String,
}

/// Native 预下单响应
#[cfg(feature = "wx-pay-native")]
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("Native预下单响应")]
pub struct NativePrepayResult {
    /// 二维码链接（用于生成支付二维码，有效期 2 小时）
    pub code_url: String,
}

/// APP 预下单响应
#[cfg(feature = "wx-pay-app")]
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("APP预下单响应")]
pub struct AppPrepayResult {
    /// 预支付交易会话标识（有效期 2 小时，APP 端用于调起微信支付 SDK）
    pub prepay_id: String,
}

/// 小程序/JSAPI 预下单响应
#[cfg(any(feature = "wx-pay-mini", feature = "wx-pay-js"))]
#[derive(Debug, Clone, Deserialize, Serialize, afast::Tag)]
#[tag("JS/小程序预下单响应")]
pub struct MiniPrepayResult {
    /// 预支付交易会话标识（有效期 2 小时，小程序端用于 wx.requestPayment）
    pub prepay_id: String,
}

/// H5 预下单
///
/// 调用微信支付 H5 下单接口，返回 `h5_url`，
/// 前端将用户重定向到该 URL 即可拉起微信支付。
///
/// - `out_trade_no`: 商户订单号 (6~32 位)
/// - `description`: 商品描述 (不超过 127 字符)
/// - `amount_total`: 订单总金额 (单位: 分)
/// - `payer_client_ip`: 用户终端 IP
/// - `h5_info_type`: 场景类型 ("Wap" / "IOS" / "Android")
/// - `scene_url`: 场景 URL (H5 网站 URL)
#[cfg(feature = "wx-pay-h5")]
#[allow(clippy::too_many_arguments)]
pub async fn prepay_h5(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_trade_no: &str,
    description: &str,
    amount_total: i64,
    payer_client_ip: &str,
    h5_info_type: &str,
    scene_url: &str,
) -> crate::Result<H5PrepayResult> {
    let private_key = get_private_key(config)?;
    let notify_url = config
        .notify_url
        .as_deref()
        .ok_or_else(|| config_missing("notify_url"))?;
    let body = serde_json::json!({
        "appid": config.app_id,
        "mchid": config.mch_id,
        "description": description,
        "out_trade_no": out_trade_no,
        "notify_url": notify_url,
        "amount": { "total": amount_total, "currency": "CNY" },
        "scene_info": {
            "payer_client_ip": payer_client_ip,
            "h5_info": { "type": h5_info_type, "app_url": scene_url }
        }
    });
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        "/v3/pay/transactions/h5",
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "prepay_h5").await);
    }
    resp.json().await.map_err(|_e| prepay_response())
}

/// Native 预下单
///
/// 调用微信支付 Native 下单接口，返回 `code_url`，
/// 商户将 `code_url` 转换为二维码展示给用户扫码支付。
#[cfg(feature = "wx-pay-native")]
pub async fn prepay_native(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_trade_no: &str,
    description: &str,
    amount_total: i64,
) -> crate::Result<NativePrepayResult> {
    let private_key = get_private_key(config)?;
    let notify_url = config
        .notify_url
        .as_deref()
        .ok_or_else(|| config_missing("notify_url"))?;
    let body = serde_json::json!({
        "appid": config.app_id,
        "mchid": config.mch_id,
        "description": description,
        "out_trade_no": out_trade_no,
        "notify_url": notify_url,
        "amount": { "total": amount_total, "currency": "CNY" }
    });
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        "/v3/pay/transactions/native",
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "prepay_native").await);
    }
    resp.json().await.map_err(|_e| prepay_response())
}

/// APP 预下单
///
/// 调用微信支付 APP 下单接口，返回 `prepay_id`，
/// APP 端使用 `prepay_id` 调起微信支付 SDK。
#[cfg(feature = "wx-pay-app")]
pub async fn prepay_app(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_trade_no: &str,
    description: &str,
    amount_total: i64,
) -> crate::Result<AppPrepayResult> {
    let private_key = get_private_key(config)?;
    let notify_url = config
        .notify_url
        .as_deref()
        .ok_or_else(|| config_missing("notify_url"))?;
    let body = serde_json::json!({
        "appid": config.app_id,
        "mchid": config.mch_id,
        "description": description,
        "out_trade_no": out_trade_no,
        "notify_url": notify_url,
        "amount": { "total": amount_total, "currency": "CNY" }
    });
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        "/v3/pay/transactions/app",
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "prepay_app").await);
    }
    resp.json().await.map_err(|_e| prepay_response())
}

/// 小程序/JSAPI 预下单
///
/// 调用微信支付 JSAPI/小程序下单接口，返回 `prepay_id`，
/// 小程序端使用 `prepay_id` 调用 `wx.requestPayment` 拉起支付。
///
/// 与 H5/Native/APP 的区别：需要传入用户 `openid`。
#[cfg(any(feature = "wx-pay-mini", feature = "wx-pay-js"))]
pub async fn prepay_js_mini(
    client: &reqwest::Client,
    config: &WxPayConfig,
    out_trade_no: &str,
    description: &str,
    amount_total: i64,
    openid: &str,
) -> crate::Result<MiniPrepayResult> {
    let private_key = get_private_key(config)?;
    let notify_url = config
        .notify_url
        .as_deref()
        .ok_or_else(|| config_missing("notify_url"))?;
    let body = serde_json::json!({
        "appid": config.app_id,
        "mchid": config.mch_id,
        "description": description,
        "out_trade_no": out_trade_no,
        "notify_url": notify_url,
        "amount": { "total": amount_total, "currency": "CNY" },
        "payer": { "openid": openid }
    });
    let resp = send_v3_request(
        client,
        &config.mch_id,
        &config.serial_no,
        &private_key,
        "POST",
        "/v3/pay/transactions/jsapi",
        &body.to_string(),
    )
    .await?;
    if !resp.status().is_success() {
        return Err(handle_error_response(resp, "prepay_js_mini").await);
    }
    resp.json().await.map_err(|_e| prepay_response())
}
