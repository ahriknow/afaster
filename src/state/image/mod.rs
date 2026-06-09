//! 图片生成模块
//!
//! 基于 `image` + `imageproc` + `ab_glyph` 封装，支持中文文字渲染。
//! 内嵌思源黑体，Builder 模式，链式调用。
//! 支持 PNG / JPEG / WebP / BMP / GIF 等多种输出格式。
//!
//! # 快速开始
//!
//! ```rust,no_run
//! use afaster::{Image, ImageTextStyle, ImageColor};
//!
//! let bytes = Image::new(800, 600)
//!     .background(ImageColor::white())
//!     .draw_rect(0, 0, 800, 80, ImageColor::rgb(33, 150, 243))
//!     .draw_text("用户报告", 20, 25, ImageTextStyle::title().color(ImageColor::white()))
//!     .draw_text("张三 - 积分: 12500", 20, 120, ImageTextStyle::body())
//!     .finish()
//!     .unwrap();
//! std::fs::write("poster.png", bytes).unwrap();
//! ```

use ab_glyph::{FontRef, PxScale};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut,
};
use imageproc::rect::Rect;

// ─── 输出格式 ──────────────────────────────────────────────────────

/// 图片输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg(u8), // 质量 1~100
    WebP,     // 仅支持无损
    Bmp,
    Gif,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Png
    }
}

// ─── 颜色 ──────────────────────────────────────────────────────────

/// RGBA 颜色
#[derive(Debug, Clone, Copy)]
pub struct ImageColor(pub u8, pub u8, pub u8, pub u8);

impl ImageColor {
    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(r, g, b, a)
    }
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b, 255)
    }
    pub fn black() -> Self {
        Self(0, 0, 0, 255)
    }
    pub fn white() -> Self {
        Self(255, 255, 255, 255)
    }
    pub fn red() -> Self {
        Self(220, 53, 69, 255)
    }
    pub fn blue() -> Self {
        Self(33, 150, 243, 255)
    }
    pub fn grey(v: u8) -> Self {
        Self(v, v, v, 255)
    }
    pub fn transparent() -> Self {
        Self(0, 0, 0, 0)
    }

    fn to_rgba(&self) -> Rgba<u8> {
        Rgba([self.0, self.1, self.2, self.3])
    }
}

// ─── 文字样式 ──────────────────────────────────────────────────────

/// 文字样式
#[derive(Debug, Clone)]
pub struct ImageTextStyle {
    pub font_size: f32,
    pub color: ImageColor,
    pub bold: bool,
}

impl Default for ImageTextStyle {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            color: ImageColor::black(),
            bold: false,
        }
    }
}

impl ImageTextStyle {
    /// 标题样式 (36pt)
    pub fn title() -> Self {
        Self {
            font_size: 36.0,
            color: ImageColor::black(),
            bold: true,
        }
    }

    /// 副标题 (28pt)
    pub fn subtitle() -> Self {
        Self {
            font_size: 28.0,
            color: ImageColor::black(),
            bold: true,
        }
    }

    /// 正文 (24pt)
    pub fn body() -> Self {
        Self::default()
    }

    /// 小字 (18pt)
    pub fn small() -> Self {
        Self {
            font_size: 18.0,
            color: ImageColor::grey(100),
            bold: false,
        }
    }

    pub fn size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }

    pub fn color(mut self, c: ImageColor) -> Self {
        self.color = c;
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

// ─── 内部元素 ──────────────────────────────────────────────────────

enum ImageElement {
    Rect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        color: ImageColor,
    },
    RoundedRect {
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: ImageColor,
    },
    Circle {
        cx: i32,
        cy: i32,
        r: i32,
        color: ImageColor,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: ImageColor,
        thickness: f32,
    },
    Text {
        text: String,
        x: i32,
        y: i32,
        style: ImageTextStyle,
    },
    Image {
        bytes: Vec<u8>,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },
}

// ─── Image 构建器 ──────────────────────────────────────────────────

/// 图片生成器（Builder 模式）
///
/// 支持 PNG / JPEG / WebP / BMP / GIF 等多种输出格式。
/// 需要通过 `font_regular()` 和 `font_bold()` 提供字体以支持文字渲染。
pub struct Image {
    width: u32,
    height: u32,
    bg_color: ImageColor,
    elements: Vec<ImageElement>,
    font_regular: Option<Vec<u8>>,
    font_bold: Option<Vec<u8>>,
}

impl Image {
    /// 创建新的图片构建器
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            bg_color: ImageColor::white(),
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

    // ── 背景 ──────────────────────────────────────────────────────

    /// 设置背景颜色
    pub fn background(mut self, color: ImageColor) -> Self {
        self.bg_color = color;
        self
    }

    // ── 绘图 ──────────────────────────────────────────────────────

    /// 绘制填充矩形
    pub fn draw_rect(mut self, x: i32, y: i32, w: u32, h: u32, color: ImageColor) -> Self {
        self.elements.push(ImageElement::Rect { x, y, w, h, color });
        self
    }

    /// 绘制圆角矩形
    pub fn draw_rounded_rect(
        mut self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: ImageColor,
    ) -> Self {
        self.elements.push(ImageElement::RoundedRect {
            x,
            y,
            w,
            h,
            radius,
            color,
        });
        self
    }

    /// 绘制圆形
    pub fn draw_circle(mut self, cx: i32, cy: i32, r: i32, color: ImageColor) -> Self {
        self.elements
            .push(ImageElement::Circle { cx, cy, r, color });
        self
    }

    /// 绘制线段
    pub fn draw_line(
        mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        color: ImageColor,
        thickness: f32,
    ) -> Self {
        self.elements.push(ImageElement::Line {
            x1,
            y1,
            x2,
            y2,
            color,
            thickness,
        });
        self
    }

    /// 绘制文字
    pub fn draw_text(mut self, text: &str, x: i32, y: i32, style: ImageTextStyle) -> Self {
        self.elements.push(ImageElement::Text {
            text: text.to_string(),
            x,
            y,
            style,
        });
        self
    }

    /// 绘制图片（PNG/JPG/WebP 等字节）
    pub fn draw_image(mut self, img_bytes: &[u8], x: i32, y: i32, w: u32, h: u32) -> Self {
        self.elements.push(ImageElement::Image {
            bytes: img_bytes.to_vec(),
            x,
            y,
            w,
            h,
        });
        self
    }

    // ── 生成图片 ──────────────────────────────────────────────────

    /// 生成 PNG 字节（默认格式）
    pub fn finish(self) -> crate::Result<Vec<u8>> {
        self.finish_with_format(ImageFormat::Png)
    }

    /// 生成指定格式的图片字节
    ///
    /// ```rust,no_run
    /// use afaster::{Image, ImageFormat};
    ///
    /// // PNG（默认）
    /// let png = Image::new(800, 600).finish()?;
    ///
    /// // JPEG（质量 90）
    /// let jpg = Image::new(800, 600)
    ///     .finish_with_format(ImageFormat::Jpeg(90))?;
    ///
    /// // WebP（无损）
    /// let webp = Image::new(800, 600)
    ///     .finish_with_format(ImageFormat::WebP)?;
    ///
    /// // BMP
    /// let bmp = Image::new(800, 600)
    ///     .finish_with_format(ImageFormat::Bmp)?;
    ///
    /// // GIF
    /// let gif = Image::new(800, 600)
    ///     .finish_with_format(ImageFormat::Gif)?;
    /// ```
    pub fn finish_with_format(self, format: ImageFormat) -> crate::Result<Vec<u8>> {
        // 创建画布
        let mut img: RgbaImage =
            ImageBuffer::from_pixel(self.width, self.height, self.bg_color.to_rgba());

        // 加载字体
        let regular_bytes = self.font_regular.as_deref().ok_or_else(|| {
            crate::Error::custom(
                51801,
                "Regular font not provided. Use Image::font_regular() to set it.",
            )
        })?;
        let bold_bytes = self.font_bold.as_deref().unwrap_or(regular_bytes);

        let font_regular = FontRef::try_from_slice(regular_bytes)
            .map_err(|e| crate::Error::custom(51801, &format!("Regular font error: {}", e)))?;
        let font_bold = FontRef::try_from_slice(bold_bytes)
            .map_err(|e| crate::Error::custom(51801, &format!("Bold font error: {}", e)))?;

        // 按顺序渲染元素
        for elem in &self.elements {
            match elem {
                ImageElement::Rect { x, y, w, h, color } => {
                    draw_filled_rect_mut(
                        &mut img,
                        Rect::at(*x, *y).of_size(*w, *h),
                        color.to_rgba(),
                    );
                }
                ImageElement::RoundedRect {
                    x,
                    y,
                    w,
                    h,
                    radius,
                    color,
                } => {
                    draw_rounded_rect(&mut img, *x, *y, *w, *h, *radius, color.to_rgba());
                }
                ImageElement::Circle { cx, cy, r, color } => {
                    draw_filled_circle_mut(&mut img, (*cx, *cy), *r, color.to_rgba());
                }
                ImageElement::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                    thickness,
                } => {
                    let steps = (*thickness).ceil() as i32;
                    for offset in -steps / 2..=steps / 2 {
                        let off = offset as f32;
                        draw_line_segment_mut(
                            &mut img,
                            (*x1 + off, *y1),
                            (*x2 + off, *y2),
                            color.to_rgba(),
                        );
                        draw_line_segment_mut(
                            &mut img,
                            (*x1, *y1 + off),
                            (*x2, *y2 + off),
                            color.to_rgba(),
                        );
                    }
                }
                ImageElement::Text { text, x, y, style } => {
                    let font = if style.bold {
                        &font_bold
                    } else {
                        &font_regular
                    };
                    let scale = PxScale::from(style.font_size);
                    draw_text_mut(&mut img, style.color.to_rgba(), *x, *y, scale, font, text);
                }
                ImageElement::Image { bytes, x, y, w, h } => {
                    draw_image_on_canvas(&mut img, bytes, *x, *y, *w, *h);
                }
            }
        }

        // 按格式编码
        encode_image(&img, self.width, self.height, format)
    }
}

// ─── 编码 ──────────────────────────────────────────────────────────

fn encode_image(
    img: &RgbaImage,
    width: u32,
    height: u32,
    format: ImageFormat,
) -> crate::Result<Vec<u8>> {
    let mut buf = Vec::new();

    match format {
        ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new(&mut buf);
            image::ImageEncoder::write_image(
                encoder,
                img.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| crate::Error::custom(51802, &format!("PNG encode error: {}", e)))?;
        }
        ImageFormat::Jpeg(quality) => {
            // JPEG 不支持 Alpha，转为 RGB
            let rgb_data: Vec<u8> = img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
            image::ImageEncoder::write_image(
                encoder,
                &rgb_data,
                width,
                height,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| crate::Error::custom(51802, &format!("JPEG encode error: {}", e)))?;
        }
        ImageFormat::WebP => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buf);
            image::ImageEncoder::write_image(
                encoder,
                img.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| crate::Error::custom(51802, &format!("WebP encode error: {}", e)))?;
        }
        ImageFormat::Bmp => {
            // BMP 不支持 Alpha，转为 RGB
            let rgb_data: Vec<u8> = img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect();
            let encoder = image::codecs::bmp::BmpEncoder::new(&mut buf);
            image::ImageEncoder::write_image(
                encoder,
                &rgb_data,
                width,
                height,
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| crate::Error::custom(51802, &format!("BMP encode error: {}", e)))?;
        }
        ImageFormat::Gif => {
            let mut encoder = image::codecs::gif::GifEncoder::new(&mut buf);
            encoder
                .encode_frame(image::Frame::new(img.clone()))
                .map_err(|e| crate::Error::custom(51802, &format!("GIF encode error: {}", e)))?;
        }
    }

    Ok(buf)
}

// ─── 辅助函数 ──────────────────────────────────────────────────────

/// 绘制圆角矩形
fn draw_rounded_rect(
    img: &mut RgbaImage,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    radius: u32,
    color: Rgba<u8>,
) {
    let r = radius.min(w / 2).min(h / 2) as i32;

    // 中间矩形
    draw_filled_rect_mut(
        img,
        Rect::at(x + r, y).of_size(w.saturating_sub(2 * r as u32), h),
        color,
    );
    // 左右矩形
    draw_filled_rect_mut(
        img,
        Rect::at(x, y + r).of_size(r as u32, h.saturating_sub(2 * r as u32)),
        color,
    );
    draw_filled_rect_mut(
        img,
        Rect::at(x + w as i32 - r, y + r).of_size(r as u32, h.saturating_sub(2 * r as u32)),
        color,
    );
    // 四个角的圆
    draw_filled_circle_mut(img, (x + r, y + r), r, color);
    draw_filled_circle_mut(img, (x + w as i32 - r - 1, y + r), r, color);
    draw_filled_circle_mut(img, (x + r, y + h as i32 - r - 1), r, color);
    draw_filled_circle_mut(img, (x + w as i32 - r - 1, y + h as i32 - r - 1), r, color);
}

/// 在画布上绘制图片（缩放到指定尺寸，支持 Alpha 混合）
fn draw_image_on_canvas(canvas: &mut RgbaImage, img_bytes: &[u8], x: i32, y: i32, w: u32, h: u32) {
    let Ok(src) = image::load_from_memory(img_bytes) else {
        return;
    };
    let src_rgba = src
        .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    for (sx, sy, pixel) in src_rgba.enumerate_pixels() {
        let dx = x + sx as i32;
        let dy = y + sy as i32;
        if dx >= 0 && dy >= 0 && (dx as u32) < canvas.width() && (dy as u32) < canvas.height() {
            // Alpha 混合
            let bg = canvas.get_pixel(dx as u32, dy as u32);
            let a = pixel[3] as f32 / 255.0;
            let blended = Rgba([
                (pixel[0] as f32 * a + bg[0] as f32 * (1.0 - a)) as u8,
                (pixel[1] as f32 * a + bg[1] as f32 * (1.0 - a)) as u8,
                (pixel[2] as f32 * a + bg[2] as f32 * (1.0 - a)) as u8,
                255,
            ]);
            canvas.put_pixel(dx as u32, dy as u32, blended);
        }
    }
}
