//! 微信 V2 消息推送基础设施
//!
//! 提供 AES-256-CBC 解密、SHA1 验签、通用通知体解析。

use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
//  GET 验签查询参数
// ═══════════════════════════════════════════════════════════════

/// 微信服务器验签查询参数
#[derive(Deserialize, afast::AFastDeserialize, afast::Tag)]
#[tag("微信服务器验签查询参数")]
pub struct WxVerifyQuery {
    pub signature: String,
    pub timestamp: String,
    pub nonce: String,
    pub echostr: String,
}

// ═══════════════════════════════════════════════════════════════
//  通用通知请求（用于提取 Event 字段）
// ═══════════════════════════════════════════════════════════════

/// 通用微信通知请求（仅用于解析 Event 字段做路由分发）
#[derive(Deserialize, Serialize)]
pub struct WxNotifyBody {
    #[serde(rename = "Event")]
    pub event: Option<String>,
    #[serde(rename = "MsgType")]
    pub msg_type: Option<String>,
    #[serde(rename = "ToUserName")]
    pub to_user_name: Option<String>,
    #[serde(rename = "FromUserName")]
    pub from_user_name: Option<String>,
    #[serde(rename = "CreateTime")]
    pub create_time: Option<i64>,
    #[serde(rename = "OpenId")]
    pub open_id: Option<String>,
    #[serde(rename = "OutTradeNo")]
    pub out_trade_no: Option<String>,
    #[serde(rename = "Env")]
    pub env: Option<i32>,
    #[serde(rename = "RetryTimes")]
    pub retry_times: Option<i32>,
    #[serde(rename = "Encrypt")]
    pub encrypt: Option<String>,
    #[serde(rename = "MsgSignature")]
    pub msg_signature: Option<String>,
    #[serde(rename = "TimeStamp")]
    pub time_stamp: Option<String>,
    #[serde(rename = "Nonce")]
    pub nonce: Option<String>,
    #[serde(flatten)]
    pub rest: std::collections::HashMap<String, serde_json::Value>,
}

impl afast::Structure for WxNotifyBody {
    fn structure() -> &'static afast::handler::TagMeta {
        use afast::handler::{FieldMeta, TagKind, TagMeta, no_structure};
        static META: TagMeta = TagMeta {
            name: "WxNotifyBody",
            desc: "微信消息推送通用通知体",
            kind: TagKind::Struct(&[
                FieldMeta {
                    name: "Event",
                    ty: "Option<String>",
                    desc: "事件类型",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "MsgType",
                    ty: "Option<String>",
                    desc: "消息类型",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "ToUserName",
                    ty: "Option<String>",
                    desc: "小程序原始 ID",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "FromUserName",
                    ty: "Option<String>",
                    desc: "openid",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "CreateTime",
                    ty: "Option<i64>",
                    desc: "消息发送时间",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "Encrypt",
                    ty: "Option<String>",
                    desc: "加密消息密文（base64）",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "MsgSignature",
                    ty: "Option<String>",
                    desc: "消息签名（加密模式）",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "TimeStamp",
                    ty: "Option<String>",
                    desc: "时间戳（加密模式）",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
                FieldMeta {
                    name: "Nonce",
                    ty: "Option<String>",
                    desc: "随机字符串（加密模式）",
                    structure: no_structure(),
                    validations: &[],
                    skip: false,
                    skip_with: "",
                },
            ]),
        };
        &META
    }
}

// ═══════════════════════════════════════════════════════════════
//  SHA1 验签
// ═══════════════════════════════════════════════════════════════

/// 验证微信服务器 GET 验签
pub fn verify_signature(
    token: &str,
    timestamp: &str,
    nonce: &str,
    signature: &str,
) -> crate::Result<()> {
    use sha1::Digest as _;
    let mut arr = [token, timestamp, nonce];
    arr.sort();
    let joined = arr.join("");
    let hash = sha1::Sha1::digest(joined.as_bytes());
    let hash_hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
    if hash_hex != signature {
        return Err(super::err::signature_mismatch());
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
//  AES-256-CBC 解密
// ═══════════════════════════════════════════════════════════════

fn decode_aes_key(encoding_key: &str) -> crate::Result<Vec<u8>> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(format!("{}=", encoding_key))
        .map_err(|e| super::err::decrypt_failed(&format!("base64 decode failed: {}", e)))
}

fn aes_decrypt(ciphertext_b64: &str, key: &[u8]) -> crate::Result<Vec<u8>> {
    use base64::Engine as _;
    use cbc::cipher::{BlockModeDecrypt, KeyIvInit};

    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(ciphertext_b64)
        .map_err(|e| {
            super::err::decrypt_failed(&format!("ciphertext base64 decode failed: {}", e))
        })?;

    if key.len() != 32 {
        return Err(super::err::decrypt_failed("key length must be 32 bytes"));
    }
    let iv = &key[..16];

    type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

    let decryptor = Aes256CbcDec::new_from_slices(key, iv)
        .map_err(|e| super::err::decrypt_failed(&format!("AES init failed: {}", e)))?;

    let mut buf = ciphertext;
    let decrypted = decryptor
        .decrypt_padded::<cbc::cipher::block_padding::Pkcs7>(&mut buf)
        .map_err(|e| super::err::decrypt_failed(&format!("PKCS7 unpad failed: {}", e)))?;

    Ok(decrypted.to_vec())
}

/// 解密微信 V2 加密消息体
pub fn decrypt_wx_message(encrypt_b64: &str, msg_secret: &str) -> crate::Result<String> {
    let key = decode_aes_key(msg_secret)?;
    let plaintext = aes_decrypt(encrypt_b64, &key)?;

    if plaintext.len() < 20 {
        return Err(super::err::decrypt_failed("decrypted data too short"));
    }
    let msg_len =
        u32::from_be_bytes([plaintext[16], plaintext[17], plaintext[18], plaintext[19]]) as usize;
    let msg_end = 20 + msg_len;
    if plaintext.len() < msg_end {
        return Err(super::err::decrypt_failed(
            "decrypted message length mismatch",
        ));
    }
    let msg_content = std::str::from_utf8(&plaintext[20..msg_end])
        .map_err(|e| super::err::decrypt_failed(&format!("message content is not UTF-8: {}", e)))?;
    Ok(msg_content.to_string())
}
