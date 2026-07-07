mod err;
use err::*;

use std::collections::BTreeMap;

use chrono::Utc;
use hmac::{Hmac, KeyInit as _, Mac};
use serde::Deserialize;
use sha1::Sha1;
use sha2::{Digest as _, Sha256};

// ═══════════════════════════════════════════════════════════════
//  公开类型
// ═══════════════════════════════════════════════════════════════

fn default_url_expire() -> u64 {
    3600
}

#[derive(Clone, Deserialize)]
pub struct Cos {
    pub secret_id: String,
    pub secret_key: String,
    pub bucket: String, // 格式: {bucketname}-{appid}
    pub region: String, // "ap-guangzhou"
    #[serde(default)]
    pub prefix: String, // 上传目录前缀，如 "materials"
    #[serde(default)]
    pub domain: String, // 自定义域名，如 "file.example.com"
    #[serde(default = "default_url_expire")]
    pub url_expire: u64, // 签名 URL 有效期（秒）
    #[serde(skip)]
    pub client: reqwest::Client,
}

type HmacSha1 = Hmac<Sha1>;

impl Cos {
    // ──────────────────────────────────────────────────────────
    //  加密工具
    // ──────────────────────────────────────────────────────────

    /// SHA-256 → 十六进制
    #[allow(dead_code)]
    fn sha256_hex(data: &[u8]) -> String {
        let hash = Sha256::digest(data);
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// SHA-1 → 十六进制
    fn sha1_hex(data: &[u8]) -> String {
        let hash = sha1::Sha1::digest(data);
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// HMAC-SHA1 → 十六进制
    fn hmac_sha1_hex(key: &[u8], data: &[u8]) -> crate::Result<String> {
        let mut mac = HmacSha1::new_from_slice(key).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51201, msg = "Tencent COS error" },  "{}", _e);
            hmac_sha1()
        })?;
        mac.update(data);
        Ok(mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect())
    }

    /// HMAC-SHA1 → 原始字节
    #[allow(dead_code)]
    fn hmac_sha1_bytes(key: &[u8], data: &[u8]) -> crate::Result<Vec<u8>> {
        let mut mac = HmacSha1::new_from_slice(key).map_err(|_e| {
            #[cfg(feature = "log")]
            tracing::error!({ code = 51201, msg = "Tencent COS error" },  "{}", _e);
            hmac_sha1()
        })?;
        mac.update(data);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    /// COS URL 编码（保留 [A-Za-z0-9\-_.~!*'()/]）
    fn cos_encode(input: &str) -> String {
        let mut out = String::with_capacity(input.len() * 3);
        for &b in input.as_bytes() {
            match b {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'~'
                | b'!'
                | b'*'
                | b'\''
                | b'('
                | b')'
                | b'/' => {
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

    /// COS 参数编码（保留 [A-Za-z0-9\-_.~]，编码 = & ; : / 等）
    fn cos_param_encode(input: &str) -> String {
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

    // ──────────────────────────────────────────────────────────
    //  域名
    // ──────────────────────────────────────────────────────────

    /// 虚拟主机风格域名，优先使用自定义域名
    fn host(&self) -> String {
        if !self.domain.is_empty() {
            return self.domain.clone();
        }
        format!("{}.cos.{}.myqcloud.com", self.bucket, self.region)
    }

    // ──────────────────────────────────────────────────────────
    //  COS 签名（q-sign-algorithm=sha1）
    //
    //  KeyTime      = {Now};{Expires}          (Unix 时间戳)
    //  SignKey      = HMAC-SHA1(SecretKey, KeyTime)
    //  HttpString   = {Method}\n{URI}\n{Params}\n{Headers}\n
    //  StringToSign = sha1\n{KeyTime}\nSHA1(HttpString)\n
    //  Signature    = HMAC-SHA1(SignKey, StringToSign)
    //
    //  extra_params:  额外查询参数，如 uploads 用于分片上传
    //  extra_headers: 需要签入的额外 Header
    // ──────────────────────────────────────────────────────────

    async fn build_signed_url(
        &self,
        method: &str,
        key: &str,
        extra_params: &[(&str, &str)],
        extra_headers: &[(&str, &str)],
    ) -> crate::Result<String> {
        let key = key.trim_start_matches('/');
        let now = Utc::now().timestamp() as u64;
        let key_time = format!("{};{}", now, now + self.url_expire);
        let host = self.host();

        // ── 1. SignKey = HMAC-SHA1(SecretKey, KeyTime) ──
        let sign_key = Self::hmac_sha1_hex(self.secret_key.as_bytes(), key_time.as_bytes())?;

        // ── 2. HttpString = Method\nURI\nParams\nHeaders\n ──
        let method_lower = method.to_lowercase();
        let uri = format!("/{}", key);

        // 构建参数（按 key 排序，保留原始大小写）
        let mut params = BTreeMap::new();
        for (k, v) in extra_params {
            params.insert(k.to_string(), v.to_string());
        }
        let params_str: String = params
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    Self::cos_param_encode(k)
                } else {
                    format!(
                        "{}={}",
                        Self::cos_param_encode(k),
                        Self::cos_param_encode(v)
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("&");

        // 构建需要签名的 headers（仅签 host，按 key 排序）
        let mut headers = BTreeMap::new();
        headers.insert("host".to_string(), host.clone());
        for (k, v) in extra_headers {
            headers.insert(k.to_lowercase(), v.to_string());
        }
        let headers_str: String = headers
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}={}",
                    Self::cos_param_encode(k),
                    Self::cos_param_encode(v)
                )
            })
            .collect::<Vec<_>>()
            .join("&");

        let http_string = format!(
            "{}\n{}\n{}\n{}\n",
            method_lower, uri, params_str, headers_str
        );

        // ── 3. StringToSign = sha1\nKeyTime\nSHA1(HttpString)\n ──
        let http_string_hash = Self::sha1_hex(http_string.as_bytes());
        let string_to_sign = format!("sha1\n{}\n{}\n", key_time, http_string_hash);

        // ── 4. Signature = HMAC-SHA1(SignKey, StringToSign) ──
        let signature = Self::hmac_sha1_hex(sign_key.as_bytes(), string_to_sign.as_bytes())?;

        // ── 5. 签名参数列表 ──
        let header_list: Vec<String> = headers.keys().cloned().collect();
        let param_list: Vec<String> = params.keys().cloned().collect();

        // ── 6. Presigned URL ──
        let authorization = format!(
            "q-sign-algorithm=sha1&\
             q-ak={}&\
             q-sign-time={}&\
             q-key-time={}&\
             q-header-list={}&\
             q-url-param-list={}&\
             q-signature={}",
            Self::cos_encode(&self.secret_id),
            Self::cos_encode(&key_time),
            Self::cos_encode(&key_time),
            Self::cos_encode(&header_list.join(";")),
            Self::cos_encode(&param_list.join(";")),
            Self::cos_encode(&signature),
        );

        let url = if extra_params.is_empty() {
            format!(
                "https://{}/{}?{}",
                host,
                Self::cos_encode(&uri),
                authorization
            )
        } else {
            // 已有额外参数时，将它们也放入 URL
            let existing_params: String = extra_params
                .iter()
                .map(|(k, v)| {
                    if v.is_empty() {
                        Self::cos_param_encode(k)
                    } else {
                        format!(
                            "{}={}",
                            Self::cos_param_encode(k),
                            Self::cos_param_encode(v)
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join("&");
            format!(
                "https://{}/{}?{}&{}",
                host,
                Self::cos_encode(&uri),
                existing_params,
                authorization
            )
        };
        Ok(url)
    }

    /// 生成下载签名 URL（GET），单个文件
    ///
    /// 客户端用 GET 请求该 URL 即可下载文件
    pub async fn get_signed_download_url(&self, key: &str) -> crate::Result<String> {
        self.build_signed_url("GET", key, &[], &[]).await
    }

    /// 根据存储的 key 生成访问 URL
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

    /// 生成上传签名 URL（PUT），单个文件
    ///
    /// 客户端用 PUT 请求该 URL，并携带签名时指定的 Content-Type Header 即可上传
    pub async fn get_signed_upload_url(
        &self,
        key: &str,
        content_type: &str,
    ) -> crate::Result<String> {
        self.build_signed_url("PUT", key, &[], &[("content-type", content_type)])
            .await
    }

    /// 批量生成下载 URL
    pub async fn get_signed_urls(&self, keys: &[&str]) -> crate::Result<Vec<String>> {
        let mut urls = Vec::with_capacity(keys.len());
        for key in keys {
            urls.push(self.get_signed_download_url(key).await?);
        }
        Ok(urls)
    }

    // ──────────────────────────────────────────────────────────
    //  图片处理 URL
    //
    //  腾讯云 COS 图片处理通过在 URL 中附加图片处理参数实现。
    //  参数格式: ?imageMogr2/resize/w_300/quality/90
    //
    //  参考文档: https://cloud.tencent.com/document/product/436/115609
    // ──────────────────────────────────────────────────────────

    /// 生成带图片处理参数的签名下载 URL
    ///
    /// # 参数
    /// - `key`: 文件 key
    /// - `params`: 图片处理参数，如 `"imageMogr2/resize/w_300"` 或 `"imageMogr2/thumbnail/!50p"`
    ///
    /// # 示例
    /// ```ignore
    /// // 缩放到 300px 宽
    /// let url = cos.get_signed_image_url("photo.jpg", "imageMogr2/resize/w_300").await?;
    ///
    /// // 缩放 + 质量
    /// let url = cos.get_signed_image_url("photo.jpg", "imageMogr2/resize/w_300/quality/90").await?;
    ///
    /// // 格式转换
    /// let url = cos.get_signed_image_url("photo.jpg", "imageMogr2/format/webp").await?;
    ///
    /// // 缩略图（百分比）
    /// let url = cos.get_signed_image_url("photo.jpg", "imageMogr2/thumbnail/!50p").await?;
    /// ```
    pub async fn get_signed_image_url(&self, key: &str, params: &str) -> crate::Result<String> {
        let prefix = self.prefix.trim_matches('/');
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", prefix, key)
        };
        // COS 图片处理参数作为查询参数签名
        self.build_signed_url("GET", &full_key, &[(params, "")], &[])
            .await
    }

    /// 生成带预定义样式的签名下载 URL
    ///
    /// 预定义样式在腾讯云 COS 控制台创建，通过样式名引用。
    ///
    /// # 参数
    /// - `key`: 文件 key
    /// - `style_name`: 样式名，如 `"thumbnail"` 或 `"avatar_s"`
    ///
    /// # 示例
    /// ```ignore
    /// let url = cos.get_signed_style_url("photo.jpg", "thumbnail").await?;
    /// ```
    pub async fn get_signed_style_url(&self, key: &str, style_name: &str) -> crate::Result<String> {
        let prefix = self.prefix.trim_matches('/');
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", prefix, key)
        };
        let param = format!("style/{}", style_name);
        self.build_signed_url("GET", &full_key, &[(&param, "")], &[])
            .await
    }

    // ──────────────────────────────────────────────────────────
    //  视频截帧 URL
    //
    //  腾讯云 COS 视频截帧通过 `ci-process=snapshot` 参数实现。
    //
    //  参考文档: https://cloud.tencent.com/document/product/460/48226
    // ──────────────────────────────────────────────────────────

    /// 生成视频截帧签名 URL
    ///
    /// 从视频中截取指定时间点的帧，返回图片 URL。
    ///
    /// # 参数
    /// - `key`: 视频文件 key
    /// - `time_sec`: 截取时间点（秒），`0` 表示封面
    /// - `width`: 输出宽度（像素），`0` 表示自动
    /// - `height`: 输出高度（像素），`0` 表示自动
    /// - `format`: 输出格式，`"jpg"` 或 `"png"`
    ///
    /// # 示例
    /// ```ignore
    /// // 截取视频封面
    /// let url = cos.get_signed_video_snapshot("video.mp4", 0, 800, 600, "jpg").await?;
    ///
    /// // 截取第 17 秒
    /// let url = cos.get_signed_video_snapshot("video.mp4", 17, 800, 600, "jpg").await?;
    /// ```
    pub async fn get_signed_video_snapshot(
        &self,
        key: &str,
        time_sec: u64,
        width: u32,
        height: u32,
        format: &str,
    ) -> crate::Result<String> {
        let prefix = self.prefix.trim_matches('/');
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{}/{}", prefix, key)
        };

        let mut params = vec![
            ("ci-process", "snapshot".to_string()),
            ("time", time_sec.to_string()),
            ("format", format.to_string()),
        ];
        if width > 0 {
            params.push(("width", width.to_string()));
        }
        if height > 0 {
            params.push(("height", height.to_string()));
        }

        let extra_params: Vec<(&str, &str)> =
            params.iter().map(|(k, v)| (*k, v.as_str())).collect();

        self.build_signed_url("GET", &full_key, &extra_params, &[])
            .await
    }

    /// 生成视频封面截帧签名 URL（简化版）
    ///
    /// # 参数
    /// - `key`: 视频文件 key
    /// - `width`: 输出宽度（像素），`0` 表示自动
    /// - `height`: 输出高度（像素），`0` 表示自动
    ///
    /// # 示例
    /// ```ignore
    /// let url = cos.get_signed_video_cover("video.mp4", 800, 0).await?;
    /// ```
    pub async fn get_signed_video_cover(
        &self,
        key: &str,
        width: u32,
        height: u32,
    ) -> crate::Result<String> {
        self.get_signed_video_snapshot(key, 0, width, height, "jpg")
            .await
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let mut instance: Self = crate::state::extract(table, "cos")?;
        instance.client = super::default_http_client();
        Ok(instance)
    }
}
