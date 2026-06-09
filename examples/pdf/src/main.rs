use afaster::pdf::{A5, Align, Pdf, PdfColor, TableStyle, TextStyle};

#[tokio::main]
async fn main() {
    println!("═══════════════════════════════════════");
    println!("  PDF 生成示例");
    println!("═══════════════════════════════════════");

    // ── 1. 简单文本 PDF ───────────────────────────────────────
    println!("\n▶ 1. 简单文本 PDF");
    let bytes = Pdf::new("简单文档")
        .text("Hello, World!", TextStyle::title(), 60.0, 250.0)
        .text(
            "这是一个使用 afaster 生成的 PDF 文档。",
            TextStyle::body(),
            15.0,
            220.0,
        )
        .text(
            "支持中文、English、数字 1234567890",
            TextStyle::body(),
            15.0,
            200.0,
        )
        .finish()
        .unwrap();
    std::fs::write("simple.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 simple.pdf ({} bytes)", bytes.len());

    // ── 2. 带样式的文本 ───────────────────────────────────────
    println!("\n▶ 2. 带样式的文本");
    let bytes = Pdf::new("样式文档")
        .text("标题样式", TextStyle::title(), 60.0, 260.0)
        .text("副标题样式", TextStyle::subtitle(), 15.0, 235.0)
        .text("正文样式 - 默认 12pt 字体", TextStyle::body(), 15.0, 215.0)
        .text("小字样式 - 9pt 灰色", TextStyle::small(), 15.0, 200.0)
        .text(
            "红色加粗文字",
            TextStyle::body().color(PdfColor::red()).bold().size(14.0),
            15.0,
            180.0,
        )
        .text(
            "居中对齐的文本",
            TextStyle::body().align(Align::Center).size(16.0),
            0.0,
            160.0,
        )
        .finish()
        .unwrap();
    std::fs::write("styled.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 styled.pdf ({} bytes)", bytes.len());

    // ── 3. 表格 ───────────────────────────────────────────────
    println!("\n▶ 3. 表格");
    let bytes = Pdf::new("销售报表")
        .text("2026年第一季度销售报表", TextStyle::title(), 30.0, 270.0)
        .table(
            &["产品名称", "数量", "单价(元)", "金额(元)"],
            &[
                &["笔记本电脑", "15", "5999.00", "89985.00"],
                &["无线鼠标", "50", "99.00", "4950.00"],
                &["机械键盘", "30", "399.00", "11970.00"],
                &["显示器", "10", "2499.00", "24990.00"],
                &["USB-Hub", "100", "49.00", "4900.00"],
            ],
        )
        .text(
            "合计: ¥136,795.00",
            TextStyle::body().bold().size(14.0),
            15.0,
            80.0,
        )
        .finish()
        .unwrap();
    std::fs::write("table.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 table.pdf ({} bytes)", bytes.len());

    // ── 4. 自定义表格样式 ─────────────────────────────────────
    println!("\n▶ 4. 自定义表格样式（简洁风格）");
    let bytes = Pdf::new("简洁报表")
        .text("简洁风格表格", TextStyle::subtitle(), 15.0, 270.0)
        .table_with_style(
            &["姓名", "部门", "职位"],
            &[
                &["张三", "技术部", "高级工程师"],
                &["李四", "产品部", "产品经理"],
                &["王五", "设计部", "UI 设计师"],
            ],
            TableStyle::minimal(),
            15.0,
            200.0,
        )
        .finish()
        .unwrap();
    std::fs::write("minimal_table.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 minimal_table.pdf ({} bytes)", bytes.len());

    // ── 5. 图形 ───────────────────────────────────────────────
    println!("\n▶ 5. 图形（矩形 + 直线）");
    let bytes = Pdf::new("图形示例")
        .text("图形绘制示例", TextStyle::title(), 50.0, 270.0)
        // 矩形
        .rect(
            15.0,
            200.0,
            80.0,
            40.0,
            Some(PdfColor::grey(0.9)),
            Some(PdfColor::black()),
        )
        .rect(
            110.0,
            200.0,
            80.0,
            40.0,
            Some(PdfColor::Rgb {
                r: 0.9,
                g: 0.95,
                b: 1.0,
            }),
            Some(PdfColor::Rgb {
                r: 0.2,
                g: 0.4,
                b: 0.8,
            }),
        )
        .text("灰色矩形", TextStyle::body().size(10.0), 30.0, 215.0)
        .text("蓝色矩形", TextStyle::body().size(10.0), 125.0, 215.0)
        // 直线
        .line(&[(15.0, 180.0), (195.0, 180.0)], PdfColor::black(), 0.5)
        .line(
            &[(15.0, 160.0), (100.0, 170.0), (195.0, 160.0)],
            PdfColor::red(),
            0.8,
        )
        .text("水平线 + 折线", TextStyle::body().size(10.0), 15.0, 145.0)
        .finish()
        .unwrap();
    std::fs::write("shapes.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 shapes.pdf ({} bytes)", bytes.len());

    // ── 6. 发票模板 ───────────────────────────────────────────
    println!("\n▶ 6. 发票模板");
    let bytes = Pdf::new("增值税普通发票")
        // 发票标题
        .text("增值税普通发票", TextStyle::title().size(22.0), 55.0, 275.0)
        // 发票信息
        .text(
            "发票代码: 044001900111",
            TextStyle::body().size(10.0),
            15.0,
            258.0,
        )
        .text(
            "发票号码: 12345678",
            TextStyle::body().size(10.0),
            130.0,
            258.0,
        )
        .text(
            "开票日期: 2026年06月02日",
            TextStyle::body().size(10.0),
            15.0,
            248.0,
        )
        .text(
            "校验码: 1234 5678 9012 3456",
            TextStyle::body().size(10.0),
            130.0,
            248.0,
        )
        // 分割线
        .line(&[(15.0, 243.0), (195.0, 243.0)], PdfColor::black(), 0.5)
        // 购买方信息
        .text("购买方", TextStyle::subtitle().size(11.0), 15.0, 233.0)
        .text(
            "名称: 某某科技有限公司",
            TextStyle::body().size(10.0),
            15.0,
            223.0,
        )
        .text(
            "纳税人识别号: 91110108MA01XXXX",
            TextStyle::body().size(10.0),
            15.0,
            213.0,
        )
        // 货物明细表
        .table_with_style(
            &[
                "货物名称",
                "规格型号",
                "数量",
                "单价",
                "金额",
                "税率",
                "税额",
            ],
            &[
                &[
                    "笔记本电脑",
                    "ThinkPad X1",
                    "2",
                    "5999.00",
                    "11998.00",
                    "13%",
                    "1559.74",
                ],
                &[
                    "无线鼠标",
                    "MX Master",
                    "5",
                    "99.00",
                    "495.00",
                    "13%",
                    "64.35",
                ],
            ],
            TableStyle::minimal().font_size(9.0),
            15.0,
            140.0,
        )
        // 合计
        .line(&[(15.0, 125.0), (195.0, 125.0)], PdfColor::black(), 0.3)
        .text(
            "合计金额: ¥12,493.00",
            TextStyle::body().bold().size(11.0),
            15.0,
            115.0,
        )
        .text(
            "合计税额: ¥1,624.09",
            TextStyle::body().bold().size(11.0),
            120.0,
            115.0,
        )
        .text(
            "价税合计(大写): 壹万肆仟壹佰壹拾柒元零玖分",
            TextStyle::body().bold().size(11.0),
            15.0,
            100.0,
        )
        .text(
            "¥14,117.09",
            TextStyle::body().bold().size(14.0).color(PdfColor::red()),
            150.0,
            100.0,
        )
        // 销售方信息
        .line(&[(15.0, 90.0), (195.0, 90.0)], PdfColor::black(), 0.5)
        .text("销售方", TextStyle::subtitle().size(11.0), 15.0, 80.0)
        .text(
            "名称: 某某商贸有限公司",
            TextStyle::body().size(10.0),
            15.0,
            70.0,
        )
        .text(
            "纳税人识别号: 91440300MA01XXXX",
            TextStyle::body().size(10.0),
            15.0,
            60.0,
        )
        // 备注
        .text(
            "备注: 此发票为示例，仅供测试",
            TextStyle::small(),
            15.0,
            40.0,
        )
        .text(
            "收款人: 张三    复核: 李四    开票人: 王五",
            TextStyle::small(),
            15.0,
            30.0,
        )
        .finish()
        .unwrap();
    std::fs::write("invoice.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 invoice.pdf ({} bytes)", bytes.len());

    // ── 7. 不同页面大小 ───────────────────────────────────────
    println!("\n▶ 7. A5 页面");
    let bytes = Pdf::new("A5文档")
        .page_size(A5)
        .text("A5 页面大小", TextStyle::title(), 30.0, 180.0)
        .text("148mm × 210mm", TextStyle::body(), 15.0, 160.0)
        .finish()
        .unwrap();
    std::fs::write("a5.pdf", &bytes).unwrap();
    println!("  ✅ 已生成 a5.pdf ({} bytes)", bytes.len());

    // ── 清理 ─────────────────────────────────────────────────
    println!("\n▶ 清理临时文件");
    let _ = std::fs::remove_file("simple.pdf");
    let _ = std::fs::remove_file("styled.pdf");
    let _ = std::fs::remove_file("table.pdf");
    let _ = std::fs::remove_file("minimal_table.pdf");
    let _ = std::fs::remove_file("shapes.pdf");
    let _ = std::fs::remove_file("invoice.pdf");
    let _ = std::fs::remove_file("a5.pdf");
    println!("  ✅ 已清理\n");

    println!("═══════════════════════════════════════");
    println!("  ✅ 所有测试通过！");
    println!("═══════════════════════════════════════");
}
