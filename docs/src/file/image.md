# 图片生成

Feature: `image` | 依赖: `image` + `imageproc` + `ab_glyph`

## 简介

通用图片生成工具。用户自行提供字体以支持中英文文字渲染。

Builder 模式，链式调用，支持矩形、圆角矩形、圆形、线段、文字、图片叠加。

支持多种输出格式：PNG、JPEG、WebP、BMP、GIF。

## 快速开始

```rust
use afaster::{Image, ImageTextStyle, ImageColor};

// 需要提供字体文件
let font = std::fs::read("fonts/SourceHanSansSC-Regular.otf")?;

let bytes = Image::new(800, 600)
    .font_regular(&font)
    .background(ImageColor::white())
    .draw_rect(0, 0, 800, 80, ImageColor::blue())
    .draw_text("用户报告", 20, 20, ImageTextStyle::title().color(ImageColor::white()))
    .draw_text("张三 - 积分: 12500", 20, 120, ImageTextStyle::body())
    .finish()?;
std::fs::write("poster.png", bytes)?;
```

## 输出格式

默认输出 PNG，可通过 `finish_with_format()` 指定格式：

```rust
use afaster::{Image, ImageFormat};

// PNG（默认，支持透明）
let png = Image::new(800, 600).finish()?;

// JPEG（质量 1~100，自动去除 Alpha 通道）
let jpg = Image::new(800, 600)
    .finish_with_format(ImageFormat::Jpeg(90))?;

// WebP（无损，支持透明）
let webp = Image::new(800, 600)
    .finish_with_format(ImageFormat::WebP)?;

// BMP（无压缩，自动去除 Alpha 通道）
let bmp = Image::new(800, 600)
    .finish_with_format(ImageFormat::Bmp)?;

// GIF（单帧）
let gif = Image::new(800, 600)
    .finish_with_format(ImageFormat::Gif)?;
```

### ImageFormat

| 格式 | 构造 | 说明 |
|------|------|------|
| `ImageFormat::Png` | 默认 | 无损，支持透明 |
| `ImageFormat::Jpeg(quality)` | `Jpeg(90)` | 有损，quality 1~100，不支持透明 |
| `ImageFormat::WebP` | `WebP` | 无损，支持透明 |
| `ImageFormat::Bmp` | `Bmp` | 无压缩，不支持透明 |
| `ImageFormat::Gif` | `Gif` | 单帧 GIF |

> JPEG 和 BMP 不支持 Alpha 通道，编码时会自动转换为 RGB。

## 创建画布

```rust
Image::new(800, 600)                              // 指定宽高（像素）

    .background(ImageColor::white())              // 默认白色
    .background(ImageColor::rgb(33, 33, 33))      // 暗色背景
    .background(ImageColor::rgba(0, 0, 0, 128))   // 半透明背景
```

## 绘图

### 矩形

```rust
Image::new(800, 600)
    // 填充矩形 (x, y, 宽, 高, 颜色)
    .draw_rect(0, 0, 800, 80, ImageColor::blue())

    // 圆角矩形 (x, y, 宽, 高, 圆角半径, 颜色)
    .draw_rounded_rect(20, 100, 300, 150, 12, ImageColor::grey(240))
```

### 圆形

```rust
Image::new(800, 600)
    // 圆形 (圆心x, 圆心y, 半径, 颜色)
    .draw_circle(400, 300, 50, ImageColor::red())
```

### 线段

```rust
Image::new(800, 600)
    // 线段 (起点x, 起点y, 终点x, 终点y, 颜色, 粗细)
    .draw_line(50.0, 100.0, 750.0, 100.0, ImageColor::grey(200), 1.0)
```

### 文字

```rust
Image::new(800, 600)
    .draw_text("标题", 20, 20, ImageTextStyle::title())
    .draw_text("正文内容", 20, 80, ImageTextStyle::body())
    .draw_text("小字说明", 20, 120, ImageTextStyle::small())
```

### 图片叠加

```rust
let avatar = std::fs::read("avatar.jpg")?;

Image::new(800, 600)
    // 图片 (字节, x, y, 宽, 高) — 自动缩放，支持 Alpha 混合
    .draw_image(&avatar, 30, 100, 80, 80)
```

## ImageTextStyle 方法

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `size(f32)` | 字体大小 (px) | 24.0 |
| `color(ImageColor)` | 文字颜色 | 黑色 |
| `bold()` | 使用粗体字体 | false |

### 预设样式

| 样式 | 字体大小 | 粗体 |
|------|----------|------|
| `ImageTextStyle::title()` | 36pt | ✅ |
| `ImageTextStyle::subtitle()` | 28pt | ✅ |
| `ImageTextStyle::body()` | 24pt | ❌ |
| `ImageTextStyle::small()` | 18pt | ❌ |

## ImageColor

```rust
ImageColor::black()                              // 黑色
ImageColor::white()                              // 白色
ImageColor::red()                                // 红色
ImageColor::blue()                               // 蓝色
ImageColor::grey(200)                            // 灰色 (0~255)
ImageColor::rgb(33, 150, 243)                    // RGB
ImageColor::rgba(33, 150, 243, 128)              // RGBA (半透明)
ImageColor::transparent()                        // 全透明
```

## 字体

字体由用户通过 `font_regular()` 提供，支持 OTF 和 TTF 格式。

推荐使用思源黑体（Source Han Sans SC），SIL Open Font License，可商用。

```rust
let font = std::fs::read("fonts/SourceHanSansSC-Regular.otf")?;

let bytes = Image::new(800, 600)
    .font_regular(&font)
    .draw_text("中文内容", 20, 20, ImageTextStyle::body())
    .finish()?;
```

> 字体需包含 CJK 字形才能正确渲染中文。`font_bold` 可选，未提供时自动使用 regular 字体。

## 海报示例

```rust
use afaster::{Image, ImageTextStyle, ImageColor};

let bytes = Image::new(800, 600)
    .background(ImageColor::white())
    // 顶部横条
    .draw_rect(0, 0, 800, 80, ImageColor::blue())
    .draw_text("用户积分报告", 20, 20,
        ImageTextStyle::title().color(ImageColor::white()))
    // 头像
    .draw_circle(80, 160, 40, ImageColor::grey(200))
    .draw_text("张三", 140, 135, ImageTextStyle::subtitle())
    .draw_text("VIP 会员", 140, 175,
        ImageTextStyle::small().color(ImageColor::blue()))
    // 数据卡片
    .draw_rounded_rect(20, 240, 240, 120, 12, ImageColor::rgb(240, 248, 255))
    .draw_text("当前积分", 40, 260, ImageTextStyle::small())
    .draw_text("12,500", 40, 295,
        ImageTextStyle::title().color(ImageColor::blue()))
    .draw_rounded_rect(280, 240, 240, 120, 12, ImageColor::rgb(255, 245, 238))
    .draw_text("累计消费", 300, 260, ImageTextStyle::small())
    .draw_text("¥58,900", 300, 295,
        ImageTextStyle::title().color(ImageColor::red()))
    // 底部
    .draw_text("数据更新时间: 2026-06-02", 20, 560, ImageTextStyle::small())
    .finish()?;
```

## 错误码

| code | 含义 |
|------|------|
| 51801 | 字体加载失败 |
| 51802 | 图片编码失败 |
