mod callback;
pub(crate) mod err;
pub use callback::{
    TencentSmsCallbackResult, TencentSmsReport, TencentSmsReportCallbackFn,
    callback as sms_report_callback_handler,
};
use err::*;

use chrono::Utc;
use hmac::{Hmac, KeyInit as _, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::AppState;
use crate::state::callbacks::IntoCallback;

// ═══════════════════════════════════════════════════════════════
//  腾讯云短信
// ═══════════════════════════════════════════════════════════════

const TENCENT_SMS_HOST: &str = "sms.tencentcloudapi.com";
const TENCENT_SMS_ENDPOINT: &str = "https://sms.tencentcloudapi.com";

/// 腾讯云短信模块
///
/// 使用 TC3-HMAC-SHA256 签名，与 COS 模块相同的签名模式。
/// API: https://cloud.tencent.com/document/api/382/55981
/// API Version: 2021-01-11
#[derive(Clone, Deserialize)]
pub struct SmsTencent {
    pub secret_id: String,
    pub secret_key: String,
    pub sdk_app_id: String, // 短信 SdkAppId
    #[serde(default = "default_sign_name")]
    pub sign_name: String, // 默认短信签名
    #[serde(default)]
    pub template_id: String, // 默认验证码模板 ID
    #[serde(skip)]
    pub client: Client,
    /// 用户注册的回执回调函数
    #[serde(skip)]
    pub(crate) sms_report_callback: Option<TencentSmsReportCallbackFn>,
    /// 回调路径
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
}

fn default_sign_name() -> String {
    String::new()
}

fn default_callback_path() -> String {
    "sms/tencent/report".to_string()
}

// ── 发送结果 ────────────────────────────────────────────────

/// 腾讯云短信发送结果
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TencentSmsResponse {
    pub request_id: String,
    pub send_status_set: Vec<SendStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SendStatus {
    pub serial_no: String,
    pub phone_number: String,
    pub fee: u32,
    pub session_context: String,
    pub code: String, // "Ok" 表示成功
    pub message: String,
    pub iso_code: String,
}

// ── 内部请求类型 ────────────────────────────────────────────

#[derive(Serialize)]
struct TencentSmsRequest {
    #[serde(rename = "PhoneNumberSet")]
    phone_number_set: Vec<String>,
    #[serde(rename = "SmsSdkAppId")]
    sms_sdk_app_id: String,
    #[serde(rename = "SignName")]
    sign_name: Option<String>,
    #[serde(rename = "TemplateId")]
    template_id: Option<String>,
    #[serde(rename = "TemplateParamSet")]
    template_param_set: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentApiResponse {
    response: TencentResponseInner,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentResponseInner {
    request_id: String,
    error: Option<TencentApiError>,
    send_status_set: Option<Vec<SendStatus>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TencentApiError {
    code: String,
    message: String,
}

// ── 实现 ────────────────────────────────────────────────────

impl Default for SmsTencent {
    fn default() -> Self {
        Self::new()
    }
}

impl SmsTencent {
    pub fn new() -> Self {
        SmsTencent {
            secret_id: String::new(),
            secret_key: String::new(),
            sdk_app_id: String::new(),
            sign_name: String::new(),
            template_id: String::new(),
            client: Client::new(),
            sms_report_callback: None,
            callback_path: default_callback_path(),
        }
    }

    // ── 加密工具 ──────────────────────────────────────────────

    /// SHA-256 → 十六进制
    fn sha256_hex(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// HMAC-SHA256 → 原始字节
    fn hmac_sha256_bytes(key: &[u8], data: &[u8]) -> crate::Result<Vec<u8>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51102, msg = "HMAC-SHA256 init failed" }, "{}", _e);
            hmac_sha256()
        })?;
        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    // ── 发送短信 ──────────────────────────────────────────────

    /// 发送短信（通用接口）
    ///
    /// - `phone_numbers` — 手机号列表（E.164 格式或纯 11 位国内号码）
    /// - `sign_name` — 短信签名，传 None 使用默认签名
    /// - `template_id` — 模板 ID，传 None 使用默认模板
    /// - `template_params` — 模板参数列表，如 `["1234"]`
    ///
    /// API: SendSms (2021-01-11)
    /// Endpoint: https://sms.tencentcloudapi.com
    pub async fn send(
        &self,
        phone_numbers: &[&str],
        sign_name: Option<&str>,
        template_id: Option<&str>,
        template_params: Option<Vec<&str>>,
    ) -> crate::Result<TencentSmsResponse> {
        let sign = sign_name.unwrap_or(&self.sign_name);
        let tpl = template_id.unwrap_or(&self.template_id);

        if sign.is_empty() || tpl.is_empty() {
            return Err(missing_credentials());
        }

        let now = Utc::now().timestamp();

        // ── 1. 构建请求体 ──
        let request = TencentSmsRequest {
            phone_number_set: phone_numbers.iter().map(|s| s.to_string()).collect(),
            sms_sdk_app_id: self.sdk_app_id.clone(),
            sign_name: Some(sign.to_string()),
            template_id: Some(tpl.to_string()),
            template_param_set: template_params.map(|v| v.iter().map(|s| s.to_string()).collect()),
        };
        let payload = serde_json::to_string(&request).map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51103, msg = "Tencent SMS request serialize failed" }, "{}", e);
            request_failed(&e.to_string())
        })?;

        // ── 2. TC3-HMAC-SHA256 签名 ──
        //
        // 参考: https://cloud.tencent.com/document/api/382/52071
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let credential_scope = format!("{}/sms/tc3_request", date);

        let canonical_request = format!(
            "POST\n\
             /\n\
             \n\
             content-type:application/json; charset=utf-8\n\
             host:{}\n\
             x-tc-action:sendsms\n\
             \n\
             content-type;host;x-tc-action\n\
             {}",
            TENCENT_SMS_HOST,
            Self::sha256_hex(payload.as_bytes())
        );

        let string_to_sign = format!(
            "TC3-HMAC-SHA256\n\
             {}\n\
             {}\n\
             {}",
            now,
            credential_scope,
            Self::sha256_hex(canonical_request.as_bytes())
        );

        let secret_date = Self::hmac_sha256_bytes(
            format!("TC3{}", self.secret_key).as_bytes(),
            date.as_bytes(),
        )?;
        let secret_service = Self::hmac_sha256_bytes(&secret_date, b"sms")?;
        let secret_signing = Self::hmac_sha256_bytes(&secret_service, b"tc3_request")?;
        let signature_bytes = Self::hmac_sha256_bytes(&secret_signing, string_to_sign.as_bytes())?;
        let signature = signature_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        let authorization = format!(
            "TC3-HMAC-SHA256 Credential={}, SignedHeaders=content-type;host;x-tc-action, Signature={}",
            credential_scope, signature
        );

        // ── 3. 发送请求 ──
        let resp = self
            .client
            .post(TENCENT_SMS_ENDPOINT)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Host", TENCENT_SMS_HOST)
            .header("X-TC-Action", "SendSms")
            .header("X-TC-Version", "2021-01-11")
            .header("X-TC-Timestamp", now.to_string())
            .header("X-TC-Region", "ap-guangzhou")
            .header("Authorization", authorization)
            .body(payload)
            .send()
            .await
            .map_err(|e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 51103, msg = "Tencent SMS request failed" }, "{}", e);
                request_failed(&e.to_string())
            })?;

        let text = resp.text().await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51104, msg = "Tencent SMS response read failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        let api_resp: TencentApiResponse = serde_json::from_str(&text).map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51104, msg = "Tencent SMS response parse failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        // 检查 API 错误
        if let Some(err) = api_resp.response.error {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 51105, msg = "Tencent SMS API error" },
                "code={}, message={}", err.code, err.message
            );
            return Err(api_error(&err.code, &err.message));
        }

        let send_status_set = api_resp.response.send_status_set.unwrap_or_default();

        // 检查每条发送状态
        for status in &send_status_set {
            if status.code != "Ok" {
                #[cfg(feature = "log")]
                tracing::error!(
                    { code = 51105, msg = "Tencent SMS send failed" },
                    "phone={}, code={}, message={}", status.phone_number, status.code, status.message
                );
                return Err(api_error(
                    &status.code,
                    &format!("{}: {}", status.phone_number, status.message),
                ));
            }
        }

        Ok(TencentSmsResponse {
            request_id: api_resp.response.request_id,
            send_status_set,
        })
    }

    /// 发送验证码短信
    ///
    /// - `phone_number` — 手机号
    /// - `code` — 验证码
    /// - `expire_minutes` — 验证码有效期（分钟），作为第二个模板变量
    pub async fn send_code(
        &self,
        phone_number: &str,
        code: &str,
        expire_minutes: Option<&str>,
    ) -> crate::Result<TencentSmsResponse> {
        let mut params = vec![code];
        if let Some(expire) = expire_minutes {
            params.push(expire);
        }
        self.send(&[phone_number], None, None, Some(params)).await
    }

    /// 发送自定义模板短信
    ///
    /// - `phone_numbers` — 手机号列表
    /// - `sign_name` — 短信签名
    /// - `template_id` — 模板 ID
    /// - `template_params` — 模板参数列表
    pub async fn send_template(
        &self,
        phone_numbers: &[&str],
        sign_name: &str,
        template_id: &str,
        template_params: Vec<&str>,
    ) -> crate::Result<TencentSmsResponse> {
        self.send(
            phone_numbers,
            Some(sign_name),
            Some(template_id),
            Some(template_params),
        )
        .await
    }

    /// 注册短信回执回调
    pub fn with_report_callback(
        mut self,
        cb: impl IntoCallback<
            (AppState, Vec<TencentSmsReport>),
            crate::Result<TencentSmsCallbackResult>,
        >,
    ) -> Self {
        self.sms_report_callback = Some(cb.into_callback());
        self
    }
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 腾讯云短信配置扩展
pub trait AFasterSmsTencentExt {
    /// 链式配置腾讯云短信
    fn with_sms_tencent(self, f: impl FnOnce(SmsTencent) -> SmsTencent) -> Self;
}

impl AFasterSmsTencentExt for crate::AFaster {
    fn with_sms_tencent(mut self, f: impl FnOnce(SmsTencent) -> SmsTencent) -> Self {
        self.state.sms_tencent = f(self.state.sms_tencent);
        self
    }
}

impl SmsTencent {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "sms_tencent")?;
        instance.client = super::default_http_client();
        Ok(instance)
    }
}
