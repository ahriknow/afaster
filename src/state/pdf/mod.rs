//! PDF 生成模块
//!
//! 基于 `printpdf 0.9` + `allsorts 0.16` 封装，内嵌思源黑体（Source Han Sans SC）支持中英文排版。
//! 使用 allsorts 进行字体解析和子集化，仅将文档中实际用到的字形嵌入 PDF，大幅减小文件体积。
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use afaster::{Pdf, TextStyle};
//!
//! let bytes = Pdf::new("发票")
//!     .text("增值税普通发票", TextStyle::title())
//!     .text("购买方: 某某科技有限公司", TextStyle::body())
//!     .finish()
//!     .unwrap();
//! std::fs::write("invoice.pdf", bytes).unwrap();
//! ```

use printpdf::font::ParsedFont;
use printpdf::serialize::PdfSaveOptions;
use printpdf::{
    Cmyk, Color, Greyscale, Line, LinePoint, Mm, Op, PaintMode, PdfDocument, PdfFontHandle,
    PdfPage, Point, Polygon, PolygonRing, Pt, RawImage, Rect, Rgb, WindingOrder, XObjectTransform,
};
use std::collections::HashSet;

// ─── 预设页面尺寸 ──────────────────────────────────────────────────
/// A4 (210 × 297 mm)
pub const A4: (Mm, Mm) = (Mm(210.0), Mm(297.0));
/// A5 (148 × 210 mm)
pub const A5: (Mm, Mm) = (Mm(148.0), Mm(210.0));
/// Letter (215.9 × 279.4 mm)
pub const LETTER: (Mm, Mm) = (Mm(215.9), Mm(279.4));
/// Legal (215.9 × 355.6 mm)
pub const LEGAL: (Mm, Mm) = (Mm(215.9), Mm(355.6));

// ─── 辅助类型 ──────────────────────────────────────────────────────

/// 颜色（RGB / CMYK / 灰度）
#[derive(Debug, Clone)]
pub enum PdfColor {
    Rgb { r: f64, g: f64, b: f64 },
    Cmyk { c: f64, m: f64, y: f64, k: f64 },
    Grey { grey: f64 },
}

impl PdfColor {
    fn to_printpdf(&self) -> Color {
        match self {
            PdfColor::Rgb { r, g, b } => {
                Color::Rgb(Rgb::new(*r as f32, *g as f32, *b as f32, None))
            }
            PdfColor::Cmyk { c, m, y, k } => {
                Color::Cmyk(Cmyk::new(*c as f32, *m as f32, *y as f32, *k as f32, None))
            }
            PdfColor::Grey { grey } => Color::Greyscale(Greyscale::new(*grey as f32, None)),
        }
    }

    /// 黑色
    pub fn black() -> Self {
        PdfColor::Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }
    }

    /// 白色
    pub fn white() -> Self {
        PdfColor::Rgb {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        }
    }

    /// 红色（发票常用）
    pub fn red() -> Self {
        PdfColor::Rgb {
            r: 1.0,
            g: 0.0,
            b: 0.0,
        }
    }

    /// 灰色
    pub fn grey(v: f64) -> Self {
        PdfColor::Grey { grey: v }
    }
}

/// 水平对齐
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// 文字样式
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font_size: f64,
    pub line_height: f64,
    pub color: PdfColor,
    pub bold: bool,
    pub align: Align,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 12.0,
            line_height: 20.0,
            color: PdfColor::black(),
            bold: false,
            align: Align::Left,
        }
    }
}

impl TextStyle {
    /// 标题样式 (24pt, 加粗)
    pub fn title() -> Self {
        Self {
            font_size: 24.0,
            line_height: 36.0,
            color: PdfColor::black(),
            bold: true,
            align: Align::Center,
        }
    }

    /// 副标题 (16pt, 加粗)
    pub fn subtitle() -> Self {
        Self {
            font_size: 16.0,
            line_height: 26.0,
            color: PdfColor::black(),
            bold: true,
            align: Align::Left,
        }
    }

    /// 正文 (12pt)
    pub fn body() -> Self {
        Self::default()
    }

    /// 小字 (9pt)
    pub fn small() -> Self {
        Self {
            font_size: 9.0,
            line_height: 14.0,
            color: PdfColor::grey(0.4),
            bold: false,
            align: Align::Left,
        }
    }

    /// 自定义字体大小
    pub fn size(mut self, s: f64) -> Self {
        self.font_size = s;
        self
    }

    /// 设置行高
    pub fn line_height(mut self, h: f64) -> Self {
        self.line_height = h;
        self
    }

    /// 设置颜色
    pub fn color(mut self, c: PdfColor) -> Self {
        self.color = c;
        self
    }

    /// 加粗
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// 设置对齐
    pub fn align(mut self, a: Align) -> Self {
        self.align = a;
        self
    }
}

/// 表格样式
#[derive(Debug, Clone)]
pub struct TableStyle {
    pub font_size: f64,
    pub header_bg: Option<PdfColor>,
    pub header_color: PdfColor,
    pub row_alt_bg: Option<PdfColor>,
    pub border_color: PdfColor,
    pub border_width: f64,
    pub padding: f64,
    pub cell_color: PdfColor,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            font_size: 10.0,
            header_bg: Some(PdfColor::grey(0.85)),
            header_color: PdfColor::black(),
            row_alt_bg: Some(PdfColor::grey(0.95)),
            border_color: PdfColor::grey(0.6),
            border_width: 0.3,
            padding: 3.0,
            cell_color: PdfColor::black(),
        }
    }
}

impl TableStyle {
    /// 简洁风格（无背景色）
    pub fn minimal() -> Self {
        Self {
            header_bg: None,
            row_alt_bg: None,
            border_color: PdfColor::grey(0.8),
            ..Default::default()
        }
    }

    /// 设置字体大小
    pub fn font_size(mut self, s: f64) -> Self {
        self.font_size = s;
        self
    }
}

// ─── 字体子集化 ──────────────────────────────────────────────────

/// 从所有 PDF 元素中收集使用的字符
fn collect_chars(elements: &[PdfElement]) -> HashSet<char> {
    let mut chars = HashSet::new();
    for elem in elements {
        match elem {
            PdfElement::Text { text, .. } => {
                chars.extend(text.chars());
            }
            PdfElement::Table { headers, rows, .. } => {
                for hdr in headers {
                    chars.extend(hdr.chars());
                }
                for row in rows {
                    for cell in row {
                        chars.extend(cell.chars());
                    }
                }
            }
            _ => {}
        }
    }
    chars
}

/// 解析字体，尝试子集化，返回 ParsedFont
///
/// 直接读取所需字体表（cmap/head/hhea），逐级容错：
/// - cmap 读取失败 → 整体失败（无法映射字符）
/// - hmtx 读取失败 → 使用默认宽度 1000
/// - 子集化失败 → 回退到完整字体嵌入（但保留 codepoint_to_glyph 映射）
fn prepare_subset_font(
    font_bytes: &[u8],
    font_index: u32,
    chars_used: &HashSet<char>,
) -> crate::Result<ParsedFont> {
    use allsorts::binary::read::ReadScope;
    use allsorts::font::read_cmap_subtable;
    use allsorts::font_data::FontData;
    use allsorts::tables::cmap::Cmap;
    use allsorts::tables::{FontTableProvider, HeadTable, HheaTable, HmtxTable, MaxpTable};
    use allsorts::tag;
    use std::collections::BTreeMap;

    // 1. 获取字体表提供者
    let scope = ReadScope::new(font_bytes);
    let font_data = scope
        .read::<FontData<'_>>()
        .map_err(|e| crate::Error::custom(51701, &format!("Font parse error: {}", e)))?;
    let provider = font_data
        .table_provider(font_index as usize)
        .map_err(|e| crate::Error::custom(51701, &format!("Font provider error: {}", e)))?;

    // 2. 读取 HEAD 表 → 字体度量（必须成功）
    let head_data = provider
        .read_table_data(tag::HEAD)
        .map_err(|e| crate::Error::custom(51701, &format!("head read error: {}", e)))?;
    let head = ReadScope::new(&head_data)
        .read::<HeadTable>()
        .map_err(|e| crate::Error::custom(51701, &format!("head parse error: {}", e)))?;

    // 3. 读取 HHEA 表 → ascent/descent/num_h_metrics（必须成功）
    let hhea_data = provider
        .read_table_data(tag::HHEA)
        .map_err(|e| crate::Error::custom(51701, &format!("hhea read error: {}", e)))?;
    let hhea = ReadScope::new(&hhea_data)
        .read::<HheaTable>()
        .map_err(|e| crate::Error::custom(51701, &format!("hhea parse error: {}", e)))?;

    // 4. 读取 CMAP 表 → 字符→GID 映射（必须成功）
    let cmap_data = provider
        .read_table_data(tag::CMAP)
        .map_err(|e| crate::Error::custom(51701, &format!("cmap read error: {}", e)))?;
    let cmap = ReadScope::new(&cmap_data)
        .read::<Cmap<'_>>()
        .map_err(|e| crate::Error::custom(51701, &format!("cmap parse error: {}", e)))?;
    let (_, subtable) = read_cmap_subtable(&cmap)
        .map_err(|e| crate::Error::custom(51701, &format!("cmap subtable error: {}", e)))?
        .ok_or_else(|| crate::Error::custom(51701, "No suitable cmap subtable found"))?;

    // 5. 字符 → GID 映射
    let mut char_to_gid: BTreeMap<char, u16> = BTreeMap::new();
    let mut glyph_ids_to_keep: Vec<u16> = vec![0]; // 始终包含 .notdef

    for &ch in chars_used {
        if let Ok(Some(gid)) = subtable.map_glyph(ch as u32) {
            if gid != 0 {
                char_to_gid.insert(ch, gid);
                glyph_ids_to_keep.push(gid);
            }
        }
    }
    glyph_ids_to_keep.sort();
    glyph_ids_to_keep.dedup();

    // 6. 读取 HMTX 表 → 字形宽度（可选，失败则用默认宽度）
    let default_width = head.units_per_em; // 通常 1000 或 2048
    let glyph_widths: BTreeMap<u16, u16> = match provider.read_table_data(tag::HMTX) {
        Ok(hmtx_data) => {
            let maxp_data = provider.read_table_data(tag::MAXP).ok();
            let num_h_metrics = hhea.num_h_metrics;
            let num_glyphs = maxp_data
                .and_then(|d| ReadScope::new(&d).read::<MaxpTable>().ok())
                .map(|m| m.num_glyphs)
                .unwrap_or(hhea.num_h_metrics);

            match ReadScope::new(&hmtx_data)
                .read_dep::<HmtxTable<'_>>((usize::from(num_glyphs), usize::from(num_h_metrics)))
            {
                Ok(hmtx) => {
                    let mut widths = BTreeMap::new();
                    for &gid in &glyph_ids_to_keep {
                        if gid != 0 {
                            let idx = gid as usize;
                            let w = if idx < num_h_metrics as usize {
                                hmtx.h_metrics
                                    .get_item(idx)
                                    .map(|m| m.advance_width)
                                    .unwrap_or(default_width)
                            } else {
                                hmtx.h_metrics
                                    .get_item(num_h_metrics as usize - 1)
                                    .map(|m| m.advance_width)
                                    .unwrap_or(default_width)
                            };
                            widths.insert(gid, w);
                        }
                    }
                    widths
                }
                Err(_) => BTreeMap::new(), // hmtx 解析失败，后面会用默认宽度
            }
        }
        Err(_) => BTreeMap::new(), // hmtx 不可读，后面会用默认宽度
    };

    // 7. 尝试子集化字体
    let subset_result = allsorts::subset::subset(
        &provider,
        &glyph_ids_to_keep,
        &allsorts::subset::SubsetProfile::Pdf,
        allsorts::subset::CmapTarget::Unicode,
    );

    let (font_bytes_final, codepoint_to_glyph, final_widths) = match subset_result {
        Ok(subset_bytes) => {
            // 子集化成功：使用新 GID 映射
            let mut orig_to_new: BTreeMap<u16, u16> = BTreeMap::new();
            for (idx, &orig_gid) in glyph_ids_to_keep.iter().enumerate() {
                orig_to_new.insert(orig_gid, idx as u16);
            }

            let mut cpg: BTreeMap<u32, u16> = BTreeMap::new();
            for (&ch, &orig_gid) in &char_to_gid {
                if let Some(&new_gid) = orig_to_new.get(&orig_gid) {
                    cpg.insert(ch as u32, new_gid);
                }
            }

            let mut widths_new: BTreeMap<u16, u16> = BTreeMap::new();
            for (&orig_gid, &width) in &glyph_widths {
                if let Some(&new_gid) = orig_to_new.get(&orig_gid) {
                    widths_new.insert(new_gid, width);
                }
            }

            (subset_bytes, cpg, widths_new)
        }
        Err(_) => {
            // 子集化失败：嵌入完整字体，使用原始 GID
            let mut cpg: BTreeMap<u32, u16> = BTreeMap::new();
            for (&ch, &gid) in &char_to_gid {
                cpg.insert(ch as u32, gid);
            }
            (font_bytes.to_vec(), cpg, glyph_widths)
        }
    };

    // 8. 确定字体类型
    let font_type = if provider.has_table(tag::CFF) {
        printpdf::font::FontType::OpenTypeCFF(())
    } else {
        printpdf::font::FontType::TrueType
    };

    // 9. 创建 ParsedFont（使用最终的字形宽度，缺失的用默认值）
    let mut parsed = ParsedFont::with_glyph_data(
        font_bytes_final,
        font_index,
        None,
        codepoint_to_glyph,
        final_widths,
        head.units_per_em,
        printpdf::font::FontMetrics {
            ascent: hhea.ascender,
            descent: hhea.descender,
        },
    );
    parsed.font_type = font_type;
    parsed.pdf_font_metrics = printpdf::font::PdfFontMetricsStub {
        units_per_em: head.units_per_em,
        x_min: head.x_min,
        y_min: head.y_min,
        x_max: head.x_max,
        y_max: head.y_max,
    };

    Ok(parsed)
}

// ─── 内部元素 ──────────────────────────────────────────────────────

enum PdfElement {
    Text {
        text: String,
        style: TextStyle,
        x: f64,
        y: f64,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        style: TableStyle,
        x: f64,
        y: f64,
        col_widths: Option<Vec<f64>>,
    },
    Image {
        bytes: Vec<u8>,
        x: f64,
        y: f64,
        width: Option<f64>,
        height: Option<f64>,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Option<PdfColor>,
        stroke: Option<PdfColor>,
        stroke_width: f64,
    },
    Line {
        points: Vec<(f64, f64)>,
        stroke: PdfColor,
        stroke_width: f64,
    },
}

// ─── Pdf 构建器 ────────────────────────────────────────────────────

/// PDF 生成器（Builder 模式）
///
/// 内嵌思源黑体，无需额外配置字体。支持文本、表格、图片、图形。
/// 使用 printpdf 0.9 的字体子集化功能，大幅减小 PDF 文件体积。
///
/// # 示例
///
/// ```rust,no_run
/// use afaster::{Pdf, TextStyle, TableStyle, PdfColor, Align};
///
/// let bytes = Pdf::new("销售发票")
///     .page_size(Pdf::A5)
///     .text("增值税普通发票", TextStyle::title())
///     .text("发票代码: 044001900111", TextStyle::body().size(10.0))
///     .text("发票号码: 12345678", TextStyle::body().size(10.0))
///     .table(
///         &["货物名称", "数量", "单价", "金额"],
///         &[
///             &["笔记本电脑", "2", "5999.00", "11998.00"],
///             &["无线鼠标", "5", "99.00", "495.00"],
///         ],
///     )
///     .text("合计: ¥12,493.00", TextStyle::body().bold().size(14.0))
///     .finish()
///     .unwrap();
/// ```
pub struct Pdf {
    title: String,
    page_size: (Mm, Mm),
    elements: Vec<PdfElement>,
    font_regular: Option<Vec<u8>>,
    font_bold: Option<Vec<u8>>,
}

impl Pdf {
    /// 创建新的 PDF 构建器
    ///
    /// 默认 A4 页面。需要通过 `font_regular()` 和 `font_bold()` 提供字体。
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            page_size: A4,
            elements: Vec::new(),
            font_regular: None,
            font_bold: None,
        }
    }

    // ── 自定义字体 ────────────────────────────────────────────────

    /// 自定义 Regular 字体（OTF/TTF 字节）
    pub fn font_regular(mut self, bytes: &[u8]) -> Self {
        self.font_regular = Some(bytes.to_vec());
        self
    }

    /// 自定义 Bold 字体（OTF/TTF 字节）
    pub fn font_bold(mut self, bytes: &[u8]) -> Self {
        self.font_bold = Some(bytes.to_vec());
        self
    }

    // ── 页面设置 ────────────────────────────────────────────────

    /// 设置页面大小
    pub fn page_size(mut self, size: (Mm, Mm)) -> Self {
        self.page_size = size;
        self
    }

    // ── 添加内容 ────────────────────────────────────────────────

    /// 添加文本
    ///
    /// 坐标原点在左下角，x 向右，y 向上。
    pub fn text(mut self, text: &str, style: TextStyle, x: f64, y: f64) -> Self {
        self.elements.push(PdfElement::Text {
            text: text.to_string(),
            style,
            x,
            y,
        });
        self
    }

    /// 添加表格
    ///
    /// 自动生成表格布局，支持表头背景、斑马纹、边框。
    pub fn table(mut self, headers: &[&str], rows: &[&[&str]]) -> Self {
        self.elements.push(PdfElement::Table {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect(),
            style: TableStyle::default(),
            x: 15.0,
            y: 0.0,
            col_widths: None,
        });
        self
    }

    /// 添加表格（自定义样式和位置）
    pub fn table_with_style(
        mut self,
        headers: &[&str],
        rows: &[&[&str]],
        style: TableStyle,
        x: f64,
        y: f64,
    ) -> Self {
        self.elements.push(PdfElement::Table {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect(),
            style,
            x,
            y,
            col_widths: None,
        });
        self
    }

    /// 添加表格（自定义列宽）
    pub fn table_with_widths(
        mut self,
        headers: &[&str],
        rows: &[&[&str]],
        col_widths: &[f64],
    ) -> Self {
        self.elements.push(PdfElement::Table {
            headers: headers.iter().map(|s| s.to_string()).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|s| s.to_string()).collect())
                .collect(),
            style: TableStyle::default(),
            x: 15.0,
            y: 0.0,
            col_widths: Some(col_widths.to_vec()),
        });
        self
    }

    /// 添加图片（PNG / JPG / BMP）
    pub fn image(mut self, img_bytes: &[u8], x: f64, y: f64) -> Self {
        self.elements.push(PdfElement::Image {
            bytes: img_bytes.to_vec(),
            x,
            y,
            width: None,
            height: None,
        });
        self
    }

    /// 添加图片（指定尺寸，单位 mm）
    pub fn image_sized(
        mut self,
        img_bytes: &[u8],
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Self {
        self.elements.push(PdfElement::Image {
            bytes: img_bytes.to_vec(),
            x,
            y,
            width: Some(width),
            height: Some(height),
        });
        self
    }

    /// 添加矩形
    pub fn rect(
        mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Option<PdfColor>,
        stroke: Option<PdfColor>,
    ) -> Self {
        self.elements.push(PdfElement::Rect {
            x,
            y,
            w,
            h,
            fill,
            stroke,
            stroke_width: 0.5,
        });
        self
    }

    /// 添加直线
    pub fn line(mut self, points: &[(f64, f64)], stroke: PdfColor, width: f64) -> Self {
        self.elements.push(PdfElement::Line {
            points: points.to_vec(),
            stroke,
            stroke_width: width,
        });
        self
    }

    // ── 生成 PDF ────────────────────────────────────────────────

    /// 生成 PDF 字节
    pub fn finish(self) -> crate::Result<Vec<u8>> {
        let regular_bytes = self.font_regular.as_deref().ok_or_else(|| {
            crate::Error::custom(
                51701,
                "Regular font not provided. Use Pdf::font_regular() to set it.",
            )
        })?;
        let bold_bytes = self.font_bold.as_deref().unwrap_or(regular_bytes);

        let (width, height) = self.page_size;
        let mut doc = PdfDocument::new(&self.title);

        // 收集使用的字符并准备字体（自动子集化或回退到完整嵌入）
        let chars_used = collect_chars(&self.elements);

        let parsed_regular = prepare_subset_font(regular_bytes, 0, &chars_used)?;
        let parsed_bold = prepare_subset_font(bold_bytes, 0, &chars_used)?;

        let font_regular_id = doc.add_font(&parsed_regular);
        let font_bold_id = doc.add_font(&parsed_bold);

        // 收集所有页面的 Ops
        let mut ops: Vec<Op> = Vec::new();

        let page_w: f64 = width.0 as f64; // mm

        // 按顺序渲染元素
        for elem in self.elements {
            match elem {
                PdfElement::Text { text, style, x, y } => {
                    render_text(
                        &mut ops,
                        &text,
                        &style,
                        x,
                        y,
                        &font_regular_id,
                        &font_bold_id,
                        page_w,
                    );
                }
                PdfElement::Table {
                    headers,
                    rows,
                    style,
                    x,
                    y,
                    col_widths,
                } => {
                    render_table(
                        &mut ops,
                        &headers,
                        &rows,
                        &style,
                        x,
                        y,
                        col_widths,
                        &font_regular_id,
                        &font_bold_id,
                        page_w,
                    );
                }
                PdfElement::Image {
                    bytes,
                    x,
                    y,
                    width: img_w,
                    height: img_h,
                } => {
                    render_image(&mut doc, &mut ops, &bytes, x, y, img_w, img_h).map_err(|e| {
                        crate::Error::custom(51703, &format!("Failed to add image: {}", e))
                    })?;
                }
                PdfElement::Rect {
                    x,
                    y,
                    w,
                    h,
                    fill,
                    stroke,
                    stroke_width,
                } => {
                    render_rect(&mut ops, x, y, w, h, fill, stroke, stroke_width);
                }
                PdfElement::Line {
                    points,
                    stroke,
                    stroke_width,
                } => {
                    render_line(&mut ops, &points, stroke, stroke_width);
                }
            }
        }

        // 创建页面并保存
        let page = PdfPage::new(width, height, ops);
        let mut warnings = Vec::new();
        let bytes = doc.with_pages(vec![page]).save(
            &PdfSaveOptions {
                subset_fonts: true,
                ..Default::default()
            },
            &mut warnings,
        );

        Ok(bytes)
    }
}

// ─── 辅助函数 ──────────────────────────────────────────────────────

/// mm 转 pt (1 mm = 72/25.4 pt ≈ 2.8346 pt)
fn mm_to_pt(mm: f64) -> Pt {
    Pt((mm * 72.0 / 25.4) as f32)
}

/// 选择字体 ID
fn select_font(
    bold: bool,
    regular: &printpdf::FontId,
    bold_font: &printpdf::FontId,
) -> printpdf::FontId {
    if bold {
        bold_font.clone()
    } else {
        regular.clone()
    }
}

// ─── 渲染函数 ──────────────────────────────────────────────────────

fn render_text(
    ops: &mut Vec<Op>,
    text: &str,
    style: &TextStyle,
    x: f64,
    y: f64,
    font_regular: &printpdf::FontId,
    font_bold: &printpdf::FontId,
    page_w: f64,
) {
    let font_id = select_font(style.bold, font_regular, font_bold);

    // 根据对齐计算 x 坐标
    let actual_x = match style.align {
        Align::Left => x,
        Align::Center => {
            let est_width = text.len() as f64 * style.font_size * 0.5 * 0.3528;
            (page_w - est_width) / 2.0
        }
        Align::Right => {
            let est_width = text.len() as f64 * style.font_size * 0.5 * 0.3528;
            page_w - x - est_width
        }
    };

    ops.push(Op::SetFillColor {
        col: style.color.to_printpdf(),
    });
    ops.push(Op::StartTextSection);
    ops.push(Op::SetFont {
        font: PdfFontHandle::External(font_id),
        size: Pt(style.font_size as f32),
    });
    ops.push(Op::SetTextCursor {
        pos: Point::new(Mm(actual_x as f32), Mm(y as f32)),
    });
    ops.push(Op::ShowText {
        items: vec![printpdf::TextItem::Text(text.to_string())],
    });
    ops.push(Op::EndTextSection);
}

fn render_table(
    ops: &mut Vec<Op>,
    headers: &[String],
    rows: &[Vec<String>],
    style: &TableStyle,
    x: f64,
    y: f64,
    col_widths: Option<Vec<f64>>,
    font_regular: &printpdf::FontId,
    font_bold: &printpdf::FontId,
    page_w: f64,
) {
    let num_cols = headers.len();
    if num_cols == 0 {
        return;
    }

    // 列宽：均分或自定义
    let avail_w = page_w - x * 2.0;
    let widths: Vec<f64> = if let Some(w) = col_widths {
        w
    } else {
        vec![avail_w / num_cols as f64; num_cols]
    };

    let row_h = style.font_size * 0.3528 + style.padding * 2.0; // pt → mm + padding

    // 从上到下绘制（PDF y 轴向上，但表格通常从上往下）
    // y 参数为表格底部 y 坐标
    let total_rows = 1 + rows.len();
    let table_height = total_rows as f64 * row_h;
    let mut cursor_y = y + table_height; // 从顶部开始

    // 绘制表头
    {
        // 表头背景
        if let Some(ref bg) = style.header_bg {
            ops.push(Op::SetFillColor {
                col: bg.to_printpdf(),
            });
            let rect_poly = filled_rect_polygon(x, cursor_y - row_h, avail_w, row_h);
            ops.push(Op::DrawPolygon { polygon: rect_poly });
        }

        // 表头文字
        ops.push(Op::SetFillColor {
            col: style.header_color.to_printpdf(),
        });
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: PdfFontHandle::External(font_bold.clone()),
            size: Pt(style.font_size as f32),
        });
        let mut cx = x + style.padding;
        for (i, hdr) in headers.iter().enumerate() {
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(cx as f32), Mm((cursor_y - row_h + style.padding) as f32)),
            });
            ops.push(Op::ShowText {
                items: vec![printpdf::TextItem::Text(hdr.clone())],
            });
            cx += widths.get(i).copied().unwrap_or(widths[0]);
        }
        ops.push(Op::EndTextSection);

        // 表头下边框
        ops.push(Op::SetOutlineColor {
            col: style.border_color.to_printpdf(),
        });
        ops.push(Op::SetOutlineThickness {
            pt: Pt(style.border_width as f32),
        });
        ops.push(Op::DrawLine {
            line: make_line(&[(x, cursor_y - row_h), (x + avail_w, cursor_y - row_h)]),
        });

        cursor_y -= row_h;
    }

    // 绘制数据行
    for (ri, row) in rows.iter().enumerate() {
        // 斑马纹背景
        if ri % 2 == 1 {
            if let Some(ref bg) = style.row_alt_bg {
                ops.push(Op::SetFillColor {
                    col: bg.to_printpdf(),
                });
                let rect_poly = filled_rect_polygon(x, cursor_y - row_h, avail_w, row_h);
                ops.push(Op::DrawPolygon { polygon: rect_poly });
            }
        }

        // 单元格文字
        ops.push(Op::SetFillColor {
            col: style.cell_color.to_printpdf(),
        });
        ops.push(Op::StartTextSection);
        ops.push(Op::SetFont {
            font: PdfFontHandle::External(font_regular.clone()),
            size: Pt(style.font_size as f32),
        });
        let mut cx = x + style.padding;
        for (ci, cell) in row.iter().enumerate() {
            ops.push(Op::SetTextCursor {
                pos: Point::new(Mm(cx as f32), Mm((cursor_y - row_h + style.padding) as f32)),
            });
            ops.push(Op::ShowText {
                items: vec![printpdf::TextItem::Text(cell.clone())],
            });
            cx += widths.get(ci).copied().unwrap_or(widths[0]);
        }
        ops.push(Op::EndTextSection);

        // 行下边框
        ops.push(Op::SetOutlineColor {
            col: style.border_color.to_printpdf(),
        });
        ops.push(Op::SetOutlineThickness {
            pt: Pt(style.border_width as f32),
        });
        ops.push(Op::DrawLine {
            line: make_line(&[(x, cursor_y - row_h), (x + avail_w, cursor_y - row_h)]),
        });

        cursor_y -= row_h;
    }

    // 外边框
    ops.push(Op::SetOutlineColor {
        col: style.border_color.to_printpdf(),
    });
    ops.push(Op::SetOutlineThickness {
        pt: Pt(style.border_width as f32),
    });
    let border = Rect {
        x: mm_to_pt(x),
        y: mm_to_pt(y),
        width: mm_to_pt(avail_w),
        height: mm_to_pt(table_height),
        mode: None,
        winding_order: None,
    };
    ops.push(Op::DrawRectangle { rectangle: border });
}

fn render_image(
    doc: &mut PdfDocument,
    ops: &mut Vec<Op>,
    img_bytes: &[u8],
    x: f64,
    y: f64,
    width: Option<f64>,
    height: Option<f64>,
) -> crate::Result<()> {
    let mut warnings = Vec::new();
    let raw_image = RawImage::decode_from_bytes(img_bytes, &mut warnings)
        .map_err(|e| crate::Error::custom(51703, &format!("Image decode error: {}", e)))?;

    let image_id = doc.add_image(&raw_image);

    let mut transform = XObjectTransform {
        translate_x: Some(mm_to_pt(x)),
        translate_y: Some(mm_to_pt(y)),
        ..Default::default()
    };

    // 如果指定了尺寸，计算缩放
    if let (Some(w), Some(h)) = (width, height) {
        // 默认 300 DPI: 1px = 0.0847mm
        // scale_x/scale_y 是倍率
        transform.scale_x = Some((w / 2.54 * 300.0 / 25.4) as f32);
        transform.scale_y = Some((h / 2.54 * 300.0 / 25.4) as f32);
    }

    ops.push(Op::UseXobject {
        id: image_id,
        transform,
    });
    Ok(())
}

fn render_rect(
    ops: &mut Vec<Op>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: Option<PdfColor>,
    stroke: Option<PdfColor>,
    stroke_width: f64,
) {
    // 确定绘制模式
    let mode = match (&fill, &stroke) {
        (Some(_), Some(_)) => PaintMode::FillStroke,
        (Some(_), None) => PaintMode::Fill,
        (None, Some(_)) => PaintMode::Stroke,
        (None, None) => PaintMode::Stroke, // 默认描边
    };

    if let Some(f) = fill {
        ops.push(Op::SetFillColor {
            col: f.to_printpdf(),
        });
    }
    if let Some(s) = stroke {
        ops.push(Op::SetOutlineColor {
            col: s.to_printpdf(),
        });
        ops.push(Op::SetOutlineThickness {
            pt: Pt(stroke_width as f32),
        });
    }

    // 使用 Polygon 绘制矩形（因为 DrawRectangle 不支持 fill/stroke）
    let rect_poly = rect_polygon_with_mode(x, y, w, h, mode);
    ops.push(Op::DrawPolygon { polygon: rect_poly });
}

fn render_line(ops: &mut Vec<Op>, points: &[(f64, f64)], stroke: PdfColor, stroke_width: f64) {
    if points.len() < 2 {
        return;
    }

    ops.push(Op::SetOutlineColor {
        col: stroke.to_printpdf(),
    });
    ops.push(Op::SetOutlineThickness {
        pt: Pt(stroke_width as f32),
    });
    ops.push(Op::DrawLine {
        line: make_line(points),
    });
}

// ─── 图形辅助函数 ──────────────────────────────────────────────────

/// 创建一条直线
fn make_line(points: &[(f64, f64)]) -> Line {
    Line {
        points: points
            .iter()
            .map(|(x, y)| LinePoint {
                p: Point::new(Mm(*x as f32), Mm(*y as f32)),
                bezier: false,
            })
            .collect(),
        is_closed: false,
    }
}

/// 创建填充矩形的 Polygon
fn filled_rect_polygon(x: f64, y: f64, w: f64, h: f64) -> Polygon {
    rect_polygon_with_mode(x, y, w, h, PaintMode::Fill)
}

/// 创建指定模式的矩形 Polygon
fn rect_polygon_with_mode(x: f64, y: f64, w: f64, h: f64, mode: PaintMode) -> Polygon {
    Polygon {
        rings: vec![PolygonRing {
            points: vec![
                LinePoint {
                    p: Point::new(Mm(x as f32), Mm(y as f32)),
                    bezier: false,
                },
                LinePoint {
                    p: Point::new(Mm((x + w) as f32), Mm(y as f32)),
                    bezier: false,
                },
                LinePoint {
                    p: Point::new(Mm((x + w) as f32), Mm((y + h) as f32)),
                    bezier: false,
                },
                LinePoint {
                    p: Point::new(Mm(x as f32), Mm((y + h) as f32)),
                    bezier: false,
                },
            ],
        }],
        mode,
        winding_order: WindingOrder::NonZero,
    }
}
