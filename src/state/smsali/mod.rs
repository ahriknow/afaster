mod callback;
pub(crate) mod err;
pub use callback::{
    AliSmsCallbackResult, AliSmsReport, AliSmsReportCallbackFn,
    callback as sms_report_callback_handler,
};
use err::*;

use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use hmac::{Hmac, KeyInit as _, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha1::Sha1;

use crate::AppState;
use crate::state::callbacks::IntoCallback;

// ═══════════════════════════════════════════════════════════════
//  阿里云短信
// ═══════════════════════════════════════════════════════════════

const ALI_SMS_ENDPOINT: &str = "https://dysmsapi.aliyuncs.com/";

/// 阿里云短信模块
///
/// 使用阿里云 RPC 签名 V1（HMAC-SHA1），与 STS 接口相同的签名方式。
/// API: https://help.aliyun.com/zh/sms/developer-reference/api-dysmsapi-2017-05-25-sendsms
#[derive(Clone, Deserialize)]
pub struct SmsAli {
    pub access_key_id: String,
    pub access_key_secret: String,
    #[serde(default = "default_sign_name")]
    pub sign_name: String, // 默认短信签名
    #[serde(default)]
    pub template_code: String, // 默认验证码模板 Code
    #[serde(skip)]
    pub client: Client,
    /// 用户注册的回执回调函数
    #[serde(skip)]
    pub(crate) sms_report_callback: Option<AliSmsReportCallbackFn>,
    /// 回调路径
    #[serde(default = "default_callback_path")]
    pub callback_path: String,
}

fn default_sign_name() -> String {
    String::new()
}

fn default_callback_path() -> String {
    "sms/ali/report".to_string()
}

// ── 发送结果 ────────────────────────────────────────────────

/// 阿里云短信发送结果
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AliSmsResponse {
    pub request_id: String,
    pub code: String, // "OK" 表示成功
    pub message: String,
    pub biz_id: Option<String>,
}

// ── 实现 ────────────────────────────────────────────────────

impl Default for SmsAli {
    fn default() -> Self {
        Self::new()
    }
}

impl SmsAli {
    pub fn new() -> Self {
        SmsAli {
            access_key_id: String::new(),
            access_key_secret: String::new(),
            sign_name: String::new(),
            template_code: String::new(),
            client: Client::new(),
            sms_report_callback: None,
            callback_path: default_callback_path(),
        }
    }

    // ── 加密工具 ──────────────────────────────────────────────

    /// 百分号编码（RFC 3986）
    fn uri_encode(s: &str) -> String {
        let mut out = String::with_capacity(s.len() * 3);
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(byte as char)
                }
                _ => out.push_str(&format!("%{:02X}", byte)),
            }
        }
        out
    }

    /// HMAC-SHA1 → Base64
    fn base64_hmac_sha1(key: &[u8], data: &str) -> crate::Result<String> {
        let mut mac = Hmac::<Sha1>::new_from_slice(key).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51101, msg = "HMAC-SHA1 init failed" }, "{}", _e);
            hmac_sha1()
        })?;
        mac.update(data.as_bytes());
        Ok(BASE64.encode(mac.finalize().into_bytes()))
    }

    /// RPC 签名（阿里云签名机制 V1，HMAC-SHA1）
    ///
    /// 参考: https://help.aliyun.com/zh/sdk/product-overview/v3-request-structure-and-signature
    fn sign_rpc_request(
        access_key_secret: &str,
        params: &BTreeMap<String, String>,
    ) -> crate::Result<String> {
        let canonicalized: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", Self::uri_encode(k), Self::uri_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let string_to_sign = format!("POST&%2F&{}", Self::uri_encode(&canonicalized));
        let signing_key = format!("{}&", access_key_secret);
        Self::base64_hmac_sha1(signing_key.as_bytes(), &string_to_sign)
    }

    // ── 发送短信 ──────────────────────────────────────────────

    /// 发送短信（通用接口）
    ///
    /// - `phone_numbers` — 逗号分隔的手机号，最多 1000 个
    /// - `sign_name` — 短信签名，传 None 使用默认签名
    /// - `template_code` — 模板 Code，传 None 使用默认模板
    /// - `template_param` — 模板变量 JSON，如 `{"code":"1234"}`
    ///
    /// API: SendSms (2017-05-25)
    /// Endpoint: https://dysmsapi.aliyuncs.com/
    pub async fn send(
        &self,
        phone_numbers: &str,
        sign_name: Option<&str>,
        template_code: Option<&str>,
        template_param: Option<&str>,
    ) -> crate::Result<AliSmsResponse> {
        let sign = sign_name.unwrap_or(&self.sign_name);
        let tpl = template_code.unwrap_or(&self.template_code);

        if sign.is_empty() {
            return Err(missing_credentials());
        }

        // ── 1. 构建系统参数 ──
        let now = Utc::now();
        let timestamp = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let mut params = BTreeMap::new();
        params.insert("Action".into(), "SendSms".into());
        params.insert("Version".into(), "2017-05-25".into());
        params.insert("Format".into(), "JSON".into());
        params.insert("AccessKeyId".into(), self.access_key_id.clone());
        params.insert("SignatureMethod".into(), "HMAC-SHA1".into());
        params.insert("Timestamp".into(), timestamp);
        params.insert("SignatureVersion".into(), "1.0".into());
        params.insert("SignatureNonce".into(), uuid_v4());

        // ── 2. 业务参数 ──
        params.insert("PhoneNumbers".into(), phone_numbers.to_string());
        params.insert("SignName".into(), sign.to_string());
        params.insert("TemplateCode".into(), tpl.to_string());
        if let Some(param) = template_param {
            params.insert("TemplateParam".into(), param.to_string());
        }

        // ── 3. 签名 ──
        let signature = Self::sign_rpc_request(&self.access_key_secret, &params)?;
        params.insert("Signature".into(), signature);

        // ── 4. 发送请求 ──
        let resp = self
            .client
            .post(ALI_SMS_ENDPOINT)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 51103, msg = "Alibaba SMS request failed" }, "{}", e);
                request_failed(&e.to_string())
            })?;

        let text = resp.text().await.map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51104, msg = "Alibaba SMS response read failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        let sms_resp: AliSmsResponse = serde_json::from_str(&text).map_err(|e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51104, msg = "Alibaba SMS response parse failed" }, "{}", e);
            response_parse_failed(&e.to_string())
        })?;

        if sms_resp.code != "OK" {
            #[cfg(feature = "log")]
            tracing::error!(
                { code = 51105, msg = "Alibaba SMS API error" },
                "code={}, message={}", sms_resp.code, sms_resp.message
            );
            return Err(api_error(&sms_resp.code, &sms_resp.message));
        }

        Ok(sms_resp)
    }

    /// 发送验证码短信
    ///
    /// - `phone_numbers` — 逗号分隔的手机号
    /// - `code` — 验证码
    pub async fn send_code(
        &self,
        phone_numbers: &str,
        code: &str,
    ) -> crate::Result<AliSmsResponse> {
        let param = format!(r#"{{"code":"{}"}}"#, code);
        self.send(phone_numbers, None, None, Some(&param)).await
    }

    /// 发送自定义模板短信
    ///
    /// - `phone_numbers` — 逗号分隔的手机号
    /// - `sign_name` — 短信签名
    /// - `template_code` — 模板 Code
    /// - `template_param` — 模板变量 JSON
    pub async fn send_template(
        &self,
        phone_numbers: &str,
        sign_name: &str,
        template_code: &str,
        template_param: &str,
    ) -> crate::Result<AliSmsResponse> {
        self.send(
            phone_numbers,
            Some(sign_name),
            Some(template_code),
            Some(template_param),
        )
        .await
    }

    /// 注册短信回执回调
    pub fn with_report_callback(
        mut self,
        cb: impl IntoCallback<(AppState, Vec<AliSmsReport>), crate::Result<AliSmsCallbackResult>>,
    ) -> Self {
        self.sms_report_callback = Some(cb.into_callback());
        self
    }
}

// ── 工具函数 ────────────────────────────────────────────────

/// 生成简易 UUID v4（用于 SignatureNonce）
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:032x}", t)
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 阿里云短信配置扩展
pub trait AFasterSmsAliExt {
    /// 链式配置阿里云短信
    fn with_sms_ali(self, f: impl FnOnce(SmsAli) -> SmsAli) -> Self;
}

impl AFasterSmsAliExt for crate::AFaster {
    fn with_sms_ali(mut self, f: impl FnOnce(SmsAli) -> SmsAli) -> Self {
        self.state.sms_ali = f(self.state.sms_ali);
        self
    }
}

impl SmsAli {
    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "sms_ali")?;
        instance.client = reqwest::Client::new();
        Ok(instance)
    }
}
