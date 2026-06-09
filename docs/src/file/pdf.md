# PDF 生成

Feature: `pdf` | 依赖: `printpdf` + `allsorts`

## 简介

PDF 文档生成工具。用户自行提供字体，支持 OTF/TTF 格式，支持中文、英文、数字排版。

使用 `allsorts` 进行字体子集化，仅将文档中实际用到的字形嵌入 PDF，大幅减小文件体积。

Builder 模式，链式调用，支持文本、表格、图片、图形。

## 快速开始

```rust
use afaster::{Pdf, TextStyle};

// 需要提供字体文件
let font_regular = std::fs::read("fonts/SourceHanSansSC-Regular.otf")?;
let font_bold = std::fs::read("fonts/SourceHanSansSC-Bold.otf")?;

let bytes = Pdf::new("发票")
    .font_regular(&font_regular)
    .font_bold(&font_bold)
    .text("增值税普通发票", TextStyle::title(), 60.0, 270.0)
    .text("购买方: 某某科技有限公司", TextStyle::body(), 15.0, 250.0)
    .text("金额: ¥12,493.00", TextStyle::body().bold(), 15.0, 230.0)
    .finish()?;
std::fs::write("invoice.pdf", bytes)?;
```

## 页面设置

```rust
use afaster::{Pdf, A4, A5, LETTER, LEGAL};

let pdf = Pdf::new("文档")
    .page_size(A4)     // 默认 210×297mm
    .page_size(A5)     // 148×210mm
    .page_size(LETTER) // 215.9×279.4mm
    .page_size(LEGAL)  // 215.9×355.6mm
    .page_size((Mm(200.0), Mm(300.0))); // 自定义
```

## 文本

```rust
use afaster::{TextStyle, PdfColor, Align};

// 预设样式
Pdf::new("doc")
    .text("标题", TextStyle::title(), x, y)       // 24pt 居中加粗
    .text("副标题", TextStyle::subtitle(), x, y)   // 16pt 加粗
    .text("正文", TextStyle::body(), x, y)          // 12pt
    .text("小字", TextStyle::small(), x, y)         // 9pt 灰色

    // 自定义样式
    .text("红色加粗", TextStyle::body()
        .color(PdfColor::red())
        .bold()
        .size(14.0), x, y)
    .text("居中", TextStyle::body()
        .align(Align::Center), x, y)
    .text("右对齐", TextStyle::body()
        .align(Align::Right), x, y)
```

### TextStyle 方法

| 方法 | 说明 | 默认值 |
|------|------|--------|
| `size(f64)` | 字体大小 (pt) | 12.0 |
| `line_height(f64)` | 行高 (pt) | 20.0 |
| `color(PdfColor)` | 文字颜色 | 黑色 |
| `bold()` | 使用粗体字体 | false |
| `align(Align)` | 对齐方式 | Left |

### PdfColor

```rust
PdfColor::black()                         // 黑色
PdfColor::white()                         // 白色
PdfColor::red()                           // 红色（发票常用）
PdfColor::grey(0.5)                       // 灰度
PdfColor::Rgb { r: 0.2, g: 0.4, b: 0.8 } // 自定义 RGB (0.0~1.0)
PdfColor::Cmyk { c: 0.0, m: 0.23, y: 0.0, k: 0.0 } // CMYK
```

### Align

```rust
Align::Left    // 左对齐（默认）
Align::Center  // 居中
Align::Right   // 右对齐
```

## 表格

```rust
// 默认样式（带表头背景、斑马纹、边框）
Pdf::new("报表")
    .table(
        &["产品", "数量", "金额"],
        &[
            &["笔记本", "2", "¥11998"],
            &["鼠标", "5", "¥495"],
        ],
    )

    // 简洁风格（无背景色）
    .table_with_style(
        &["姓名", "部门"],
        &[&["张三", "技术部"]],
        TableStyle::minimal(),
        15.0, 200.0,  // x, y 坐标
    )

    // 自定义列宽
    .table_with_widths(
        &["名称", "描述", "价格"],
        &[&["商品A", "描述文字", "99.00"]],
        &[50.0, 80.0, 30.0],  // 列宽 mm
    )
```

### TableStyle

```rust
TableStyle::default()   // 默认：带背景色和边框
TableStyle::minimal()   // 简洁：无背景色，浅灰边框

TableStyle::default()
    .font_size(9.0)                    // 字体大小
```

## 图片

支持 PNG / JPG / BMP 格式。

```rust
let img_bytes = std::fs::read("logo.png")?;

Pdf::new("文档")
    .image(&img_bytes, 15.0, 250.0)                    // 原始尺寸
    .image_sized(&img_bytes, 15.0, 200.0, 30.0, 30.0) // 指定宽高 (mm)
```

## 图形

```rust
Pdf::new("文档")
    // 矩形 (x, y, w, h, 填充色, 边框色)
    .rect(15.0, 200.0, 80.0, 40.0,
        Some(PdfColor::grey(0.9)),
        Some(PdfColor::black()))

    // 直线 (点坐标, 颜色, 线宽)
    .line(&[(15.0, 180.0), (195.0, 180.0)],
        PdfColor::black(), 0.5)

    // 折线
    .line(&[(15.0, 160.0), (100.0, 170.0), (195.0, 160.0)],
        PdfColor::red(), 0.8)
```

## 完整发票示例

```rust
use afaster::{Pdf, TextStyle, TableStyle, PdfColor};

let bytes = Pdf::new("增值税普通发票")
    .text("增值税普通发票", TextStyle::title().size(22.0), 55.0, 275.0)
    .text("发票代码: 044001900111", TextStyle::body().size(10.0), 15.0, 258.0)
    .text("发票号码: 12345678", TextStyle::body().size(10.0), 130.0, 258.0)
    .line(&[(15.0, 243.0), (195.0, 243.0)], PdfColor::black(), 0.5)
    .text("购买方: 某某科技有限公司", TextStyle::body().size(10.0), 15.0, 233.0)
    .table_with_style(
        &["货物名称", "数量", "单价", "金额", "税率", "税额"],
        &[
            &["笔记本电脑", "2", "5999.00", "11998.00", "13%", "1559.74"],
            &["无线鼠标", "5", "99.00", "495.00", "13%", "64.35"],
        ],
        TableStyle::minimal().font_size(9.0),
        15.0, 140.0,
    )
    .text("价税合计: ¥14,117.09",
        TextStyle::body().bold().size(14.0).color(PdfColor::red()),
        150.0, 100.0)
    .finish()?;
```

## 字体

字体由用户通过 `font_regular()` / `font_bold()` 提供，支持 OTF 和 TTF 格式。

推荐使用思源黑体（Source Han Sans SC），SIL Open Font License，可商用：

| 字体 | 用途 |
|------|------|
| SourceHanSansSC-Regular.otf | 正文 |
| SourceHanSansSC-Bold.otf | 加粗 |

覆盖范围：中文（简体）、英文、数字、常用标点。

```rust
let font_regular = std::fs::read("fonts/SourceHanSansSC-Regular.otf")?;
let font_bold = std::fs::read("fonts/SourceHanSansSC-Bold.otf")?;

let bytes = Pdf::new("文档")
    .font_regular(&font_regular)
    .font_bold(&font_bold)
    .text("内容", TextStyle::body(), 15.0, 250.0)
    .finish()?;
```

> **注意：** `font_regular` 是必须的。如果未提供 `font_bold`，将自动使用 regular 字体代替。

使用 `allsorts` 进行字体子集化，仅嵌入文档中实际用到的字形，大幅减小 PDF 体积。

## 坐标系

PDF 坐标原点在**左下角**，x 向右，y 向上，单位 mm。

A4 页面常用坐标参考：

```
(0,297) ───────────────── (210,297)
  │                           │
  │     标题 y≈270            │
  │     正文 y≈250            │
  │     表格 y≈100~200        │
  │     页脚 y≈20             │
  │                           │
(0,0) ──────────────────── (210,0)
```

## 错误码

| code | 含义 |
|------|------|
| 51701 | 字体加载/解析失败 |
| 51702 | 字体子集化失败 |
| 51703 | 图片处理失败 |
