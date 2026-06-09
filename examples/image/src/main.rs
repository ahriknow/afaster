use afaster::image::{Image, ImageColor, ImageFormat, ImageTextStyle};

fn main() {
    println!("═══════════════════════════════════════");
    println!("  图片生成示例（多格式）");
    println!("═══════════════════════════════════════");

    // ── 1. 海报（PNG） ───────────────────────────────────────
    println!("\n▶ 1. 海报 (PNG)");
    let bytes = Image::new(800, 600)
        .background(ImageColor::white())
        .draw_rect(0, 0, 800, 80, ImageColor::blue())
        .draw_text(
            "用户积分报告",
            20,
            20,
            ImageTextStyle::title().color(ImageColor::white()),
        )
        .draw_circle(80, 160, 40, ImageColor::grey(200))
        .draw_text("张三", 140, 135, ImageTextStyle::subtitle())
        .draw_text(
            "VIP 会员",
            140,
            175,
            ImageTextStyle::small().color(ImageColor::blue()),
        )
        .draw_rounded_rect(20, 240, 240, 120, 12, ImageColor::rgb(240, 248, 255))
        .draw_text("当前积分", 40, 260, ImageTextStyle::small())
        .draw_text(
            "12,500",
            40,
            295,
            ImageTextStyle::title().color(ImageColor::blue()),
        )
        .draw_rounded_rect(280, 240, 240, 120, 12, ImageColor::rgb(255, 245, 238))
        .draw_text("累计消费", 300, 260, ImageTextStyle::small())
        .draw_text(
            "¥58,900",
            300,
            295,
            ImageTextStyle::title().color(ImageColor::red()),
        )
        .draw_rounded_rect(540, 240, 240, 120, 12, ImageColor::rgb(240, 255, 240))
        .draw_text("会员等级", 560, 260, ImageTextStyle::small())
        .draw_text(
            "Lv.5",
            560,
            295,
            ImageTextStyle::title().color(ImageColor::rgb(76, 175, 80)),
        )
        .draw_text("数据更新时间: 2026-06-02", 20, 560, ImageTextStyle::small())
        .finish()
        .unwrap();
    std::fs::write("poster.png", &bytes).unwrap();
    println!("  ✅ poster.png ({} bytes)", bytes.len());

    // ── 2. JPEG ──────────────────────────────────────────────
    println!("\n▶ 2. JPEG (质量 90)");
    let bytes = Image::new(400, 200)
        .background(ImageColor::white())
        .draw_rounded_rect(20, 20, 360, 160, 12, ImageColor::rgb(33, 150, 243))
        .draw_text(
            "JPEG 格式",
            120,
            80,
            ImageTextStyle::subtitle().color(ImageColor::white()),
        )
        .draw_text(
            "有损压缩，适合照片",
            100,
            130,
            ImageTextStyle::body().color(ImageColor::white()),
        )
        .finish_with_format(ImageFormat::Jpeg(90))
        .unwrap();
    std::fs::write("output.jpg", &bytes).unwrap();
    println!("  ✅ output.jpg ({} bytes)", bytes.len());

    // ── 3. WebP 无损 ─────────────────────────────────────────
    println!("\n▶ 3. WebP (无损)");
    let bytes = Image::new(400, 200)
        .background(ImageColor::rgb(33, 33, 33))
        .draw_rounded_rect(20, 20, 360, 160, 16, ImageColor::rgb(66, 66, 66))
        .draw_text(
            "WebP 格式",
            120,
            80,
            ImageTextStyle::subtitle().color(ImageColor::white()),
        )
        .draw_text(
            "现代格式，体积更小",
            100,
            130,
            ImageTextStyle::body().color(ImageColor::grey(180)),
        )
        .finish_with_format(ImageFormat::WebP)
        .unwrap();
    std::fs::write("output.webp", &bytes).unwrap();
    println!("  ✅ output.webp ({} bytes)", bytes.len());

    // ── 4. BMP ───────────────────────────────────────────────
    println!("\n▶ 4. BMP");
    let bytes = Image::new(200, 200)
        .background(ImageColor::white())
        .draw_circle(100, 100, 80, ImageColor::red())
        .draw_text(
            "BMP",
            70,
            90,
            ImageTextStyle::body().color(ImageColor::white()),
        )
        .finish_with_format(ImageFormat::Bmp)
        .unwrap();
    std::fs::write("output.bmp", &bytes).unwrap();
    println!("  ✅ output.bmp ({} bytes)", bytes.len());

    // ── 5. GIF ───────────────────────────────────────────────
    println!("\n▶ 5. GIF");
    let bytes = Image::new(200, 200)
        .background(ImageColor::rgb(240, 240, 240))
        .draw_rounded_rect(30, 30, 140, 140, 20, ImageColor::rgb(76, 175, 80))
        .draw_text(
            "GIF",
            70,
            90,
            ImageTextStyle::subtitle().color(ImageColor::white()),
        )
        .finish_with_format(ImageFormat::Gif)
        .unwrap();
    std::fs::write("output.gif", &bytes).unwrap();
    println!("  ✅ output.gif ({} bytes)", bytes.len());

    // ── 清理 ─────────────────────────────────────────────────
    println!("\n▶ 清理临时文件");
    for f in &[
        "poster.png",
        "output.jpg",
        "output.webp",
        "output.bmp",
        "output.gif",
    ] {
        let _ = std::fs::remove_file(f);
    }
    println!("  ✅ 已清理");

    println!("\n═══════════════════════════════════════");
    println!("  ✅ 所有测试通过！");
    println!("═══════════════════════════════════════");
}
