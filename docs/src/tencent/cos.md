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
| `get_signed_image_url` | `key: &str, params: &str` | `Result<String>` | 生成带图片处理参数的签名 URL |
| `get_signed_style_url` | `key: &str, style_name: &str` | `Result<String>` | 生成带预定义样式的签名 URL |
| `get_signed_video_snapshot` | `key, time_sec, w, h, format` | `Result<String>` | 视频截帧签名 URL |
| `get_signed_video_cover` | `key, w, h` | `Result<String>` | 视频封面截帧（简化版） |

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

## 图片处理

COS 通过数据万象提供图片处理能力，在 URL 后附加处理参数即可。

> 📖 参考文档：[图片处理机制介绍](https://cloud.tencent.com/document/product/436/115609)

### 实时处理参数

使用 `get_signed_image_url` 附加图片处理参数：

```rust
// 缩放到 300px 宽
let url = state.cos.get_signed_image_url("photo.jpg", "imageMogr2/resize/w_300").await?;

// 缩放 + 质量
let url = state.cos.get_signed_image_url("photo.jpg", "imageMogr2/resize/w_300/quality/90").await?;

// 格式转换
let url = state.cos.get_signed_image_url("photo.jpg", "imageMogr2/format/webp").await?;

// 缩略图（百分比）
let url = state.cos.get_signed_image_url("photo.jpg", "imageMogr2/thumbnail/!50p").await?;

// 链式处理
let url = state.cos.get_signed_image_url("photo.jpg", "imageMogr2/resize/w_300/quality/90/format/webp").await?;
```

常用处理参数：

| 参数 | 说明 | 示例 |
|------|------|------|
| `imageMogr2/resize` | 图片缩放 | `imageMogr2/resize/w_300` / `imageMogr2/resize/w_300/h_200` |
| `imageMogr2/quality` | 质量变换 | `imageMogr2/quality/90` |
| `imageMogr2/format` | 格式转换 | `imageMogr2/format/webp` |
| `imageMogr2/crop` | 自定义裁剪 | `imageMogr2/crop/w_100/h_100/x_10/y_10` |
| `imageMogr2/rotate` | 旋转 | `imageMogr2/rotate/90` |
| `imageMogr2/blur` | 模糊效果 | `imageMogr2/blur/r_5/s_2` |
| `imageMogr2/roundPic` | 圆角裁剪 | `imageMogr2/roundPic/r_20` |
| `imageMogr2/thumbnail` | 缩略图 | `imageMogr2/thumbnail/!50p` / `imageMogr2/thumbnail/200x200` |
| `imageMogr2/interlace` | 渐进显示 | `imageMogr2/interlace/1` |
| `imageMogr2/averageHue` | 获取主色调 | `imageMogr2/averageHue` |

多个参数可链式拼接：`imageMogr2/resize/w_300/quality/90/format/webp`

### 预定义样式

在腾讯云 COS 控制台创建预定义样式后，通过样式名引用：

```rust
// 使用缩略图样式
let url = state.cos.get_signed_style_url("photo.jpg", "thumbnail").await?;

// 使用头像样式
let url = state.cos.get_signed_style_url("photo.jpg", "avatar_s").await?;
```

## 视频截帧

COS 支持从视频中截取指定时间点的帧作为图片。

> 📖 参考文档：[视频截帧](https://cloud.tencent.com/document/product/460/48226)

### 获取视频封面

```rust
// 获取视频封面（第 0 秒），800px 宽
let url = state.cos.get_signed_video_cover("video.mp4", 800, 0).await?;

// 获取视频封面，指定宽高
let url = state.cos.get_signed_video_cover("video.mp4", 800, 600).await?;
```

### 截取指定时间点

```rust
// 截取第 17 秒处的帧，800x600，JPG 格式
let url = state.cos
    .get_signed_video_snapshot("video.mp4", 17, 800, 600, "jpg")
    .await?;

// 输出 PNG 格式
let url = state.cos
    .get_signed_video_snapshot("video.mp4", 5, 0, 0, "png")
    .await?;
```

### 参数说明

| 参数 | 说明 |
|------|------|
| `time_sec` | 截取时间点（秒），`0` = 封面 |
| `width` | 输出宽度（像素），`0` = 自动 |
| `height` | 输出高度（像素），`0` = 自动 |
| `format` | 输出格式：`"jpg"` 或 `"png"` |

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
