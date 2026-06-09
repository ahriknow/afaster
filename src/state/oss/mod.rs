#[allow(dead_code)]
mod err;
use err::*;

use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::Utc;
use hmac::{Hmac, KeyInit as _, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest as _, Sha256};

// ═══════════════════════════════════════════════════════════════
//  公开类型
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StsToken {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub security_token: String,
    pub expiration: String,
}

fn default_endpoint() -> String {
    String::new()
}
fn default_url_expire() -> u64 {
    3600
}
fn default_sts_expire() -> u64 {
    3600
}

#[derive(Clone, Deserialize)]
pub struct Oss {
    #[serde(default = "default_endpoint")]
    pub endpoint: String, // 可选，为空时从 region 推导: oss-{region}.aliyuncs.com
    pub access_key_id: String,
    pub access_key_secret: String,
    #[serde(alias = "bucket_name")]
    pub bucket: String,
    #[serde(default = "default_url_expire")]
    pub url_expire: u64, // 签名 URL 有效期（秒）
    #[serde(default = "default_sts_expire")]
    pub sts_expire: u64, // STS 临时凭证有效期（秒）
    #[serde(default)]
    pub role_arn: String, // RAM 角色 ARN，空字符串表示不使用 STS
    pub region: String, // "cn-hangzhou"
    #[serde(default)]
    pub prefix: String, // 上传目录（文件夹），如 "materials"
    #[serde(default)]
    pub domain: String, // 自定义域名，如 "file.kuaizhunyun.com"
    #[serde(skip)]
    pub client: reqwest::Client,
}

// ═══════════════════════════════════════════════════════════════
//  STS API 内部类型
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StsResponse {
    #[allow(dead_code)]
    request_id: String,
    credentials: StsCredentials,
    #[serde(flatten)]
    #[serde(default)]
    #[allow(dead_code)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StsCredentials {
    access_key_id: String,
    access_key_secret: String,
    security_token: String,
    expiration: String,
    #[serde(flatten)]
    #[serde(default)]
    #[allow(dead_code)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

impl Oss {
    // ──────────────────────────────────────────────────────────
    //  加密工具
    // ──────────────────────────────────────────────────────────

    /// SHA-256 → 十六进制
    fn sha256_hex(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// HMAC-SHA256
    fn hmac_sha256(key: &[u8], data: &[u8]) -> crate::Result<Vec<u8>> {
        let mut mac = HmacSha256::new_from_slice(key).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50401, msg = "Aliyun OSS error" },  "{}", _e);
            hmac_sha256()
        })?;
        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    /// HMAC-SHA1 → Base64（仅 STS RPC 签名使用）
    fn base64_hmac_sha1(key: &[u8], data: &str) -> crate::Result<String> {
        let mut mac = HmacSha1::new_from_slice(key).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50402, msg = "Aliyun OSS error" },  "{}", _e);
            hmac_sha1()
        })?; // HMAC-SHA1 accepts any key length
        mac.update(data.as_bytes());
        Ok(BASE64.encode(mac.finalize().into_bytes()))
    }

    /// RFC 3986 percent-encode（保留 [A-Za-z0-9\-_.~]）
    fn uri_encode(input: &str) -> String {
        let mut out = String::with_capacity(input.len() * 3);
        for &b in input.as_bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    out.push(b as char);
                }
                _ => {
                    out.push('%');
                    out.push_str(&format!("{:02X}", b));
                }
            }
        }
        out
    }

    /// 路径编码（保留 /）
    fn uri_encode_path(path: &str) -> String {
        path.split('/')
            .map(|s| Self::uri_encode(s))
            .collect::<Vec<_>>()
            .join("/")
    }

    /// 生成不重复的 nonce
    fn generate_nonce() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        format!("{}{:09}", d.as_secs(), d.subsec_nanos())
    }

    // ──────────────────────────────────────────────────────────
    //  域名
    // ──────────────────────────────────────────────────────────

    /// 解析 endpoint，若未配置则从 region 推导
    fn resolve_endpoint(&self) -> String {
        if self.endpoint.is_empty() {
            format!("oss-{}.aliyuncs.com", self.region)
        } else {
            self.endpoint
                .strip_prefix("https://")
                .or(self.endpoint.strip_prefix("http://"))
                .unwrap_or(&self.endpoint)
                .trim_end_matches('/')
                .to_string()
        }
    }

    /// 虚拟主机风格，优先使用自定义域名
    fn host(&self) -> String {
        if !self.domain.is_empty() {
            return self.domain.clone();
        }
        let ep = self.resolve_endpoint();
        format!("{}.{}", self.bucket, ep)
    }

    // ──────────────────────────────────────────────────────────
    //  V4 签名密钥派生（4 层 HMAC-SHA256 链）
    //
    //  DateKey              = HMAC-SHA256("aliyun_v4" + SK, Date)
    //  DateRegionKey        = HMAC-SHA256(DateKey, Region)
    //  DateRegionServiceKey = HMAC-SHA256(DateRegionKey, "oss")
    //  SigningKey           = HMAC-SHA256(DateRegionServiceKey, "aliyun_v4_request")
    // ──────────────────────────────────────────────────────────

    fn build_signing_key(&self, date: &str) -> crate::Result<Vec<u8>> {
        let k_date = Self::hmac_sha256(
            format!("aliyun_v4{}", self.access_key_secret).as_bytes(),
            date.as_bytes(),
        )?;
        let k_region = Self::hmac_sha256(&k_date, self.region.as_bytes())?;
        let k_service = Self::hmac_sha256(&k_region, b"oss")?;
        Self::hmac_sha256(&k_service, b"aliyun_v4_request")
    }

    // ──────────────────────────────────────────────────────────
    //  签名 URL（OSS4-HMAC-SHA256）
    //
    //  用 AK/SK 直接签名，不依赖 STS，权限精确到单个对象路径
    //
    //  Canonical Request =
    //    {METHOD}\n/{key}\n{QueryString}\n{Headers}\n{SignedHeaders}\nUNSIGNED-PAYLOAD
    //
    //  StringToSign =
    //    OSS4-HMAC-SHA256\n{Timestamp}\n{Scope}\nSHA256(CanonicalRequest)
    //
    //  extra_headers: 需要签入请求的额外 Header，如 PUT 需要 content-type
    // ──────────────────────────────────────────────────────────

    async fn build_signed_url(
        &self,
        method: &str,
        key: &str,
        extra_headers: &[(&str, &str)],
    ) -> crate::Result<String> {
        let key = key.trim_start_matches('/');
        let now = Utc::now();
        let date = now.format("%Y%m%d").to_string();
        let timestamp = now.format("%Y%m%dT%H%M%SZ").to_string();
        let host = self.host();

        // Credential scope: {Date}/{Region}/oss/aliyun_v4_request
        let credential_scope = format!("{}/{}/oss/aliyun_v4_request", date, self.region);

        // ── 1. Canonical Headers（按名称字典序排列）──
        // V4 presigned URL 不需要签名 host header（host 已隐含在 URL 中）
        // 仅签名 extra_headers（如 PUT 时的 content-type）
        let mut headers: Vec<(&str, String)> = Vec::new();
        for (name, value) in extra_headers {
            headers.push((name, value.to_string()));
        }
        headers.sort_by(|a, b| a.0.cmp(b.0));

        let mut header_lines = Vec::new();
        let mut header_names = Vec::new();
        for (name, value) in &headers {
            header_lines.push(format!("{}:{}", name, value));
            header_names.push(*name);
        }
        let canonical_headers = if header_lines.is_empty() {
            String::new()
        } else {
            header_lines.join("\n") + "\n"
        };
        let signed_headers = header_names.join(";");

        // ── 2. Canonical Query String ──
        let mut query_params = BTreeMap::new();
        query_params.insert(
            "x-oss-credential".to_string(),
            format!("{}/{}", self.access_key_id, credential_scope),
        );
        query_params.insert("x-oss-date".to_string(), timestamp.clone());
        query_params.insert("x-oss-expires".to_string(), self.url_expire.to_string());
        query_params.insert(
            "x-oss-signature-version".to_string(),
            "OSS4-HMAC-SHA256".to_string(),
        );
        if !signed_headers.is_empty() {
            query_params.insert("x-oss-signed-headers".to_string(), signed_headers.clone());
        }

        let canonical_query: String = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", Self::uri_encode(k), Self::uri_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // ── 3. Canonical Request ──
        // V4 签名的 Canonical URI 始终将 Bucket 作为路径的一部分
        // 即使实际请求使用 virtual-hosted 风格
        let canonical_uri = format!("/{}/{}", self.bucket, key);
        // 注意: 预签名 URL 的 Canonical Request 格式不包含 Additional Headers 行
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n\nUNSIGNED-PAYLOAD",
            method.to_uppercase(),
            Self::uri_encode_path(&canonical_uri),
            canonical_query,
            canonical_headers,
        );

        // ── 4. String to Sign ──
        let cr_hash = Self::sha256_hex(canonical_request.as_bytes());
        let string_to_sign = format!(
            "OSS4-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, cr_hash,
        );

        // ── 5. Signing Key → Signature ──
        let signing_key = self.build_signing_key(&date)?;
        let signature: String = {
            let mut mac = HmacSha256::new_from_slice(&signing_key).map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50403, msg = "Aliyun OSS error" },  "{}", _e);
                sign()
            })?;
            mac.update(string_to_sign.as_bytes());
            mac.finalize()
                .into_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect()
        };

        // ── 6. Presigned URL ──
        let mut url = format!(
            "https://{}/{}?\
x-oss-credential={}&\
x-oss-date={}&\
x-oss-expires={}&\
x-oss-signature-version=OSS4-HMAC-SHA256",
            host,
            Self::uri_encode_path(key),
            Self::uri_encode(&format!("{}/{}", self.access_key_id, credential_scope)),
            Self::uri_encode(&timestamp),
            self.url_expire,
        );
        if !signed_headers.is_empty() {
            url.push_str(&format!(
                "&x-oss-signed-headers={}",
                Self::uri_encode(&signed_headers),
            ));
        }
        url.push_str(&format!("&x-oss-signature={}", signature));
        Ok(url)
    }

    /// 生成 V4 签名下载 URL（GET），单个文件
    ///
    /// 客户端用 GET 请求该 URL 即可下载文件，无需额外 Header
    pub async fn get_signed_download_url(&self, key: &str) -> crate::Result<String> {
        self.build_signed_url("GET", key, &[]).await
    }

    /// 根据存储的 key（如 `abc123.png`）生成访问 URL
    ///
    /// 如果配置了 `prefix`，自动拼接为 `{prefix}/{key}` 再签名
    pub async fn get_access_url(&self, stored_key: &str) -> crate::Result<String> {
        let prefix = self.prefix.trim_matches('/');
        let key = if prefix.is_empty() {
            stored_key.to_string()
        } else {
            format!("{}/{}", prefix, stored_key)
        };
        self.get_signed_download_url(&key).await
    }

    /// 生成 V4 签名上传 URL（PUT），单个文件
    ///
    /// 客户端用 PUT 请求该 URL，并携带签名时指定的 Content-Type Header 即可上传
    pub async fn get_signed_upload_url(
        &self,
        key: &str,
        content_type: &str,
    ) -> crate::Result<String> {
        self.build_signed_url("PUT", key, &[("content-type", content_type)])
            .await
    }

    /// 批量生成下载 URL（兼容旧接口）
    pub async fn get_signed_urls(&self, keys: &[&str]) -> crate::Result<Vec<String>> {
        let mut urls = Vec::with_capacity(keys.len());
        for key in keys {
            urls.push(self.get_signed_download_url(key).await?);
        }
        Ok(urls)
    }

    // ──────────────────────────────────────────────────────────
    //  STS 临时凭证（RPC 签名 v1 — HMAC-SHA1）
    //
    //  STS AssumeRole API 使用阿里云 RPC 签名机制，
    //  固定为 HMAC-SHA1，不支持 V4 签名。
    // ──────────────────────────────────────────────────────────

    fn sign_rpc_request(&self, params: &BTreeMap<String, String>) -> crate::Result<String> {
        let canonicalized: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", Self::uri_encode(k), Self::uri_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        let string_to_sign = format!("POST&%2F&{}", Self::uri_encode(&canonicalized));
        let signing_key = format!("{}&", self.access_key_secret);
        Self::base64_hmac_sha1(signing_key.as_bytes(), &string_to_sign)
    }

    /// 申请 STS 临时凭证
    ///
    /// `path` — 限制上传目录前缀，如 "uploads/2024"
    /// 返回的凭证仅允许 PutObject 到 `{bucket}/{path}*`
    pub async fn get_sts_token(&self, path: &str) -> crate::Result<StsToken> {
        if self.role_arn.is_empty() {
            return Err(sts_no_role_arn());
        }
        let path = path.trim_matches('/');

        let policy = serde_json::json!({
            "Version": "1",
            "Statement": [{
                "Effect": "Allow",
                "Action": ["oss:PutObject"],
                "Resource": [
                    format!("acs:oss:*:*:{}/{}*", self.bucket, path)
                ]
            }]
        });

        let mut params = BTreeMap::new();
        params.insert("Action".into(), "AssumeRole".into());
        params.insert("RoleArn".into(), self.role_arn.clone());
        params.insert("RoleSessionName".into(), "oss-upload".into());
        params.insert("Policy".into(), policy.to_string());
        params.insert("DurationSeconds".into(), self.sts_expire.to_string());
        params.insert("Format".into(), "JSON".into());
        params.insert("Version".into(), "2015-04-01".into());
        params.insert("RegionId".into(), self.region.clone());
        params.insert("AccessKeyId".into(), self.access_key_id.clone());
        params.insert("SignatureMethod".into(), "HMAC-SHA1".into());
        params.insert(
            "Timestamp".into(),
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        );
        params.insert("SignatureVersion".into(), "1.0".into());
        params.insert("SignatureNonce".into(), Self::generate_nonce());

        let signature = self.sign_rpc_request(&params)?;
        params.insert("Signature".into(), signature);

        let resp = Client::new()
            .post("https://sts.aliyuncs.com/")
            .form(&params)
            .send()
            .await
            .map_err(|_e| {
                #[cfg(feature = "log")]
                tracing::error!({ code = 50404, msg = "Aliyun OSS error" },  "{}", _e);
                sts_request()
            })?;

        if !resp.status().is_success() {
            let _body = resp.text().await.unwrap_or_default();
            #[cfg(feature = "log")]
            tracing::error!({ code = 50405, msg = "Aliyun OSS error" },  "STS API error: {}", _body);
            return Err(sts_response());
        }

        let sts_resp: StsResponse = resp.json().await.map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 50406, msg = "Aliyun OSS error" },  "{}", _e);
            sts_parse()
        })?;

        Ok(StsToken {
            access_key_id: sts_resp.credentials.access_key_id,
            access_key_secret: sts_resp.credentials.access_key_secret,
            security_token: sts_resp.credentials.security_token,
            expiration: sts_resp.credentials.expiration,
        })
    }

    pub async fn get_sts_tokens(&self, paths: &[&str]) -> crate::Result<Vec<StsToken>> {
        let mut tokens = Vec::with_capacity(paths.len());
        for path in paths {
            tokens.push(self.get_sts_token(path).await?);
        }
        Ok(tokens)
    }
}
