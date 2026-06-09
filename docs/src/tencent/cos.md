# 腾讯云 COS 对象存储

Feature: `cos`

> 📖 官方文档：<https://cloud.tencent.com/document/product/436>

## 配置

```toml
[cos]
secret_id = ""       # 必填
secret_key = ""      # 必填
bucket = ""          # 必填, 格式: {bucketname}-{appid}
region = ""          # 必填, 如 "ap-guangzhou"
prefix = ""          # 上传目录前缀
domain = ""          # 自定义域名
url_expire = 3600    # 签名 URL 有效期（秒）
```

## API

### Cos

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `get_signed_download_url` | `key: &str` | `Result<String>` | 生成下载签名 URL |
| `get_signed_upload_url` | `key: &str, content_type: &str` | `Result<String>` | 生成上传签名 URL |
| `get_access_url` | `stored_key: &str` | `Result<String>` | 生成访问 URL（自动拼接 prefix） |
| `get_signed_urls` | `keys: &[&str]` | `Result<Vec<String>>` | 批量生成下载 URL |

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 51201 | HMAC-SHA1 init failed | HMAC-SHA1 初始化失败 |

## 使用示例

```rust
// 生成下载 URL
let url = state.cos.get_signed_download_url("abc123.png").await?;

// 生成上传 URL（指定 Content-Type）
let url = state.cos.get_signed_upload_url("abc123.png", "image/png").await?;

// 自动拼接 prefix 生成访问 URL
let url = state.cos.get_access_url("abc123.png").await?;

// 批量生成下载 URL
let urls = state.cos.get_signed_urls(&["a.png", "b.png"]).await?;
```

## 签名算法

COS 使用基于 HMAC-SHA1 的签名机制：

```
KeyTime      = {Now};{Expires}              (Unix 时间戳)
SignKey      = HMAC-SHA1(SecretKey, KeyTime)
HttpString   = {Method}\n{URI}\n{Params}\n{Headers}\n
StringToSign = sha1\n{KeyTime}\nSHA1(HttpString)\n
Signature    = HMAC-SHA1(SignKey, StringToSign)
```

签名参数通过 URL query string 传递（预签名 URL 方式）。
