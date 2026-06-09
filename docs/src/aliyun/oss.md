# 阿里云 OSS

Feature: `oss`

> 📖 官方文档：<https://help.aliyun.com/zh/oss/>

## 配置

```toml
[oss]
access_key_id = ""                    # 必填
access_key_secret = ""                # 必填
region = "cn-hangzhou"                # 必填
bucket = ""                           # 必填
endpoint = ""                         # 可选, 空则自动推导
url_expire = 3600                     # 签名 URL 有效期（秒）
sts_expire = 3600                     # STS 临时凭证有效期（秒）
role_arn = ""                         # RAM 角色 ARN, 空则禁用 STS
prefix = ""                           # 上传目录前缀
domain = ""                           # 自定义域名
```

## API

### Oss

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_signed_download_url` | `key: &str` | `Result<String>` | 生成下载签名 URL |
| `get_signed_upload_url` | `key: &str, content_type: &str` | `Result<String>` | 生成上传签名 URL |
| `get_access_url` | `stored_key: &str` | `Result<String>` | 生成访问 URL（自动拼接 prefix） |
| `get_signed_urls` | `keys: &[&str]` | `Result<Vec<String>>` | 批量生成下载 URL |
| `get_sts_token` | `path: &str` | `Result<StsToken>` | 申请 STS 临时凭证 |
| `get_sts_tokens` | `paths: &[&str]` | `Result<Vec<StsToken>>` | 批量申请 STS |

### StsToken

```rust
pub struct StsToken {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub security_token: String,
    pub expiration: String,
}
```

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 40401 | STS role_arn not configured | STS 未配置 role_arn |
| 50401 | HMAC-SHA256 init failed | HMAC-SHA256 初始化失败 |
| 50402 | HMAC-SHA1 init failed | HMAC-SHA1 初始化失败 |
| 50403 | Signature calculation failed | 签名计算失败 |
| 50404 | STS request failed | STS 请求发送失败 |
| 50405 | STS response error | STS 响应错误 |
| 50406 | STS response parse failed | STS 响应解析失败 |

## 使用示例

```rust
// 生成下载 URL
let url = state.oss.get_signed_download_url("abc123.png").await?;

// 生成上传 URL（指定 Content-Type）
let url = state.oss.get_signed_upload_url("abc123.png", "image/png").await?;

// 自动拼接 prefix 生成访问 URL
let url = state.oss.get_access_url("abc123.png").await?;

// 申请 STS 临时凭证（需配置 role_arn）
let sts = state.oss.get_sts_token("uploads/2024").await?;
```

## 签名算法

- 下载/上传 URL: OSS4-HMAC-SHA256
- STS: RPC 签名 v1 (HMAC-SHA1)
