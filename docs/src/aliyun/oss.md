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
| `get_signed_image_url` | `key: &str, process: &str` | `Result<String>` | 生成带图片处理参数的签名 URL |
| `get_signed_style_url` | `key: &str, style_name: &str` | `Result<String>` | 生成带预定义样式的签名 URL |
| `get_signed_video_snapshot` | `key, time_ms, w, h, format, fast` | `Result<String>` | 视频截帧签名 URL |
| `get_signed_video_cover` | `key, w, h` | `Result<String>` | 视频封面截帧（简化版） |
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

## 图片处理

OSS 支持在下载 URL 中附加图片处理参数，对图片进行实时处理。

### 实时处理参数

使用 `get_signed_image_url` 附加实时处理参数：

```rust
// 缩放到 300px 宽（等比缩放）
let url = state.oss.get_signed_image_url("photo.jpg", "image/resize,w_300").await?;

// 缩放到指定宽高
let url = state.oss.get_signed_image_url("photo.jpg", "image/resize,w_300,h_200").await?;

// 缩放 + 质量变换（链式处理）
let url = state.oss.get_signed_image_url("photo.jpg", "image/resize,w_300/quality,q_90").await?;

// 格式转换
let url = state.oss.get_signed_image_url("photo.jpg", "image/format,webp").await?;

// 圆角裁剪
let url = state.oss.get_signed_image_url("photo.jpg", "image/rounded-corners,r_20").await?;

// 模糊效果
let url = state.oss.get_signed_image_url("photo.jpg", "image/blur,r_5,s_2").await?;
```

常用处理参数：

| 参数 | 说明 | 示例 |
|------|------|------|
| `resize` | 图片缩放 | `image/resize,w_300` / `image/resize,w_300,h_200` |
| `quality` | 质量变换 | `quality,q_90` |
| `format` | 格式转换 | `format,webp` |
| `crop` | 自定义裁剪 | `crop,w_100,h_100,x_10,y_10` |
| `rotate` | 旋转 | `rotate,90` |
| `blur` | 模糊效果 | `blur,r_5,s_2` |
| `rounded-corners` | 圆角矩形 | `rounded-corners,r_20` |
| `watermark` | 水印 | `watermark,text_dGVzdA==` |
| `bright` | 亮度 | `bright,50` |
| `sharpen` | 锐化 | `sharpen,100` |

多个参数可链式拼接：`image/resize,w_300/quality,q_90/format,webp`

### 预定义样式

在阿里云 OSS 控制台创建预定义样式后，通过样式名引用：

```rust
// 使用缩略图样式
let url = state.oss.get_signed_style_url("photo.jpg", "thumbnail").await?;

// 使用头像样式
let url = state.oss.get_signed_style_url("photo.jpg", "avatar_s").await?;

// 使用自定义样式
let url = state.oss.get_signed_style_url("photo.jpg", "watermark_v2").await?;
```

> 📖 详细参数说明请参考[阿里云 OSS 图片处理文档](https://help.aliyun.com/zh/oss/user-guide/overview-17/)

## 视频截帧

OSS 支持从视频中截取指定时间点的帧作为图片。支持 H264/H265 编码。

> 📖 参考文档：[视频单帧截取](https://help.aliyun.com/zh/oss/user-guide/video-snapshots)

### 获取视频封面

```rust
// 获取视频封面（第 0 帧），800px 宽
let url = state.oss.get_signed_video_cover("video.mp4", 800, 0).await?;

// 获取视频封面，指定宽高
let url = state.oss.get_signed_video_cover("video.mp4", 800, 600).await?;
```

### 截取指定时间点

```rust
// 截取第 17 秒处的帧，800x600，JPG 格式
let url = state.oss
    .get_signed_video_snapshot("video.mp4", 17000, 800, 600, "jpg", false)
    .await?;

// fast 模式：截取最近关键帧（更快但不精确）
let url = state.oss
    .get_signed_video_snapshot("video.mp4", 7000, 800, 600, "jpg", true)
    .await?;

// 输出 PNG 格式
let url = state.oss
    .get_signed_video_snapshot("video.mp4", 5000, 0, 0, "png", false)
    .await?;
```

### 参数说明

| 参数 | 说明 |
|------|------|
| `time_ms` | 截取时间点（毫秒），`0` = 封面 |
| `width` | 输出宽度（像素），`0` = 自动 |
| `height` | 输出高度（像素），`0` = 自动 |
| `format` | 输出格式：`"jpg"` 或 `"png"` |
| `fast` | `true` = 截取最近关键帧，`false` = 精确时间点 |

## 签名算法

- 下载/上传 URL: OSS4-HMAC-SHA256
- STS: RPC 签名 v1 (HMAC-SHA1)
