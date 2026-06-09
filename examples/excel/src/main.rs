use afaster::excel::{CellValue, Excel, Format, RowReader, RowWriter};

#[tokio::main]
async fn main() {
    println!("═══════════════════════════════════════");
    println!("  Excel / CSV 导入导出示例");
    println!("═══════════════════════════════════════");

    // ── 1. 写入简单 Excel ───────────────────────────────────
    println!("\n▶ 1. 写入简单 Excel（write）");
    let bytes = Excel::write(&[
        &["Name", "Age", "City"],
        &["Alice", "25", "Beijing"],
        &["Bob", "30", "Shanghai"],
        &["Charlie", "35", "Guangzhou"],
    ])
    .unwrap();

    std::fs::write("simple.xlsx", &bytes).unwrap();
    println!("  ✅ 已写入 simple.xlsx ({} bytes)", bytes.len());

    // ── 2. 读取 Excel ───────────────────────────────────────
    println!("\n▶ 2. 读取 Excel（read_path）");
    let data = Excel::read_path("simple.xlsx", None).unwrap();
    for row in &data {
        let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
        println!("  | {}", cells.join(" | "));
    }

    // ── 3. 写入带类型的 Excel ────────────────────────────────
    println!("\n▶ 3. 写入带类型的 Excel（write_typed）");
    let typed_data = vec![
        vec![
            CellValue::String("Product".into()),
            CellValue::String("Price".into()),
            CellValue::String("InStock".into()),
            CellValue::String("SKU".into()),
        ],
        vec![
            CellValue::String("Laptop".into()),
            CellValue::Float(5999.99),
            CellValue::Bool(true),
            CellValue::String("LP001".into()),
        ],
        vec![
            CellValue::String("Mouse".into()),
            CellValue::Int(99),
            CellValue::Bool(true),
            CellValue::String("MS002".into()),
        ],
        vec![
            CellValue::String("Monitor".into()),
            CellValue::Float(2499.50),
            CellValue::Bool(false),
            CellValue::String("MN003".into()),
        ],
    ];

    let bytes = Excel::write_typed(&typed_data, Some("Products")).unwrap();
    std::fs::write("typed.xlsx", &bytes).unwrap();
    println!("  ✅ 已写入 typed.xlsx ({} bytes)", bytes.len());

    // ── 4. 读取带表头的 Excel ────────────────────────────────
    println!("\n▶ 4. 读取带表头（read_with_headers）");
    let (headers, rows) = Excel::read_with_headers("typed.xlsx", None).unwrap();
    println!("  表头: {:?}", headers);
    for row in &rows {
        let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
        println!("  | {}", cells.join(" | "));
    }

    // ── 5. 读取为 JSON ──────────────────────────────────────
    println!("\n▶ 5. 读取为 JSON（read_to_json）");
    let json = Excel::read_to_json("typed.xlsx", None).unwrap();
    for item in &json {
        println!("  {}", serde_json::to_string(item).unwrap());
    }

    // ── 6. 写入多工作表 ──────────────────────────────────────
    println!("\n▶ 6. 写入多工作表（write_multi）");
    let sheets = vec![
        (
            "Users",
            vec![
                vec![
                    CellValue::String("ID".into()),
                    CellValue::String("Name".into()),
                    CellValue::String("Email".into()),
                ],
                vec![
                    CellValue::Int(1),
                    CellValue::String("Alice".into()),
                    CellValue::String("alice@example.com".into()),
                ],
                vec![
                    CellValue::Int(2),
                    CellValue::String("Bob".into()),
                    CellValue::String("bob@example.com".into()),
                ],
            ],
        ),
        (
            "Orders",
            vec![
                vec![
                    CellValue::String("OrderID".into()),
                    CellValue::String("UserID".into()),
                    CellValue::String("Amount".into()),
                ],
                vec![
                    CellValue::Int(1001),
                    CellValue::Int(1),
                    CellValue::Float(299.99),
                ],
                vec![
                    CellValue::Int(1002),
                    CellValue::Int(2),
                    CellValue::Float(599.00),
                ],
            ],
        ),
    ];

    let bytes = Excel::write_multi(&sheets).unwrap();
    std::fs::write("multi.xlsx", &bytes).unwrap();
    println!("  ✅ 已写入 multi.xlsx ({} bytes, 2 sheets)", bytes.len());

    // ── 7. 读取所有工作表 ────────────────────────────────────
    println!("\n▶ 7. 读取所有工作表（read_all）");
    let all = Excel::read_all("multi.xlsx").unwrap();
    for (name, sheet_data) in &all {
        println!("  📄 Sheet: {}", name);
        for row in sheet_data {
            let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
            println!("    | {}", cells.join(" | "));
        }
    }

    // ── 8. 从字节读取 ────────────────────────────────────────
    println!("\n▶ 8. 从字节读取（read_bytes）");
    let file_bytes = std::fs::read("simple.xlsx").unwrap();
    let data = Excel::read_bytes(&file_bytes, None).unwrap();
    println!("  ✅ 从字节读取了 {} 行", data.len());

    // ── 9. 写入带表头的 Excel ────────────────────────────────
    println!("\n▶ 9. 写入带表头（write_with_headers）");
    let headers = vec!["Language", "Rating", "Popular"];
    let rows = vec![
        vec![
            CellValue::String("Rust".into()),
            CellValue::Int(10),
            CellValue::Bool(true),
        ],
        vec![
            CellValue::String("Python".into()),
            CellValue::Int(9),
            CellValue::Bool(true),
        ],
        vec![
            CellValue::String("Go".into()),
            CellValue::Int(8),
            CellValue::Bool(true),
        ],
    ];
    let bytes = Excel::write_with_headers(&headers, &rows, Some("Languages")).unwrap();
    std::fs::write("headers.xlsx", &bytes).unwrap();
    println!("  ✅ 已写入 headers.xlsx ({} bytes)", bytes.len());

    // ═══════════════════════════════════════════════════════════
    //  CSV 测试
    // ═══════════════════════════════════════════════════════════

    // ── 10. 写入 CSV（write_as）──────────────────────────────
    println!("\n▶ 10. 写入 CSV（write_as）");
    let bytes = Excel::write_as(
        &[
            &["Name", "Age", "City"],
            &["Alice", "25", "Beijing"],
            &["Bob", "30", "Shanghai"],
        ],
        Format::Csv,
    )
    .unwrap();
    std::fs::write("data.csv", &bytes).unwrap();
    println!("  ✅ 已写入 data.csv ({} bytes)", bytes.len());
    println!("  内容:\n{}", String::from_utf8_lossy(&bytes));

    // ── 11. 自动识别读取 CSV ─────────────────────────────────
    println!("▶ 11. 自动识别读取 CSV（read_path）");
    let data = Excel::read_path("data.csv", None).unwrap();
    for row in &data {
        let cells: Vec<String> = row.iter().map(|c| format!("{:?}", c)).collect();
        println!("  | {}", cells.join(" | "));
    }

    // ── 12. 带类型写入 CSV ───────────────────────────────────
    println!("\n▶ 12. 带类型写入 CSV（write_typed_as）");
    let typed_csv = vec![
        vec![
            CellValue::String("Product".into()),
            CellValue::String("Price".into()),
            CellValue::String("InStock".into()),
        ],
        vec![
            CellValue::String("Laptop".into()),
            CellValue::Float(5999.99),
            CellValue::Bool(true),
        ],
        vec![
            CellValue::String("Mouse".into()),
            CellValue::Int(99),
            CellValue::Bool(false),
        ],
    ];
    let bytes = Excel::write_typed_as(&typed_csv, Format::Csv, None).unwrap();
    std::fs::write("products.csv", &bytes).unwrap();
    println!("  ✅ 已写入 products.csv ({} bytes)", bytes.len());
    println!("  内容:\n{}", String::from_utf8_lossy(&bytes));

    // ── 13. CSV 读取验证类型自动推断 ─────────────────────────
    println!("▶ 13. CSV 类型自动推断验证（read_path）");
    let data = Excel::read_path("products.csv", None).unwrap();
    for row in &data {
        let cells: Vec<String> = row.iter().map(|c| format!("{:?}", c)).collect();
        println!("  | {}", cells.join(" | "));
    }

    // ── 14. CSV 带表头写入 ───────────────────────────────────
    println!("\n▶ 14. CSV 带表头写入（write_with_headers_as）");
    let headers = vec!["Language", "Rating"];
    let rows = vec![
        vec![CellValue::String("Rust".into()), CellValue::Int(10)],
        vec![CellValue::String("Python".into()), CellValue::Int(9)],
    ];
    let bytes = Excel::write_with_headers_as(&headers, &rows, Format::Csv, None).unwrap();
    std::fs::write("langs.csv", &bytes).unwrap();
    println!("  ✅ 已写入 langs.csv");
    println!("  内容:\n{}", String::from_utf8_lossy(&bytes));

    // ── 15. CSV 多 sheet（仅写第一个）────────────────────────
    println!("▶ 15. CSV 多 sheet（write_multi_as，仅写第一个）");
    let bytes = Excel::write_multi_as(&sheets, Format::Csv).unwrap();
    println!("  ✅ CSV 输出 ({} bytes)，仅包含第一个 sheet", bytes.len());
    println!("  内容:\n{}", String::from_utf8_lossy(&bytes));

    // ── 16. read_all CSV（单 sheet）──────────────────────────
    println!("▶ 16. read_all CSV（返回单个 Sheet1）");
    let all = Excel::read_all("data.csv").unwrap();
    for (name, sheet_data) in &all {
        println!("  📄 Sheet: {} ({} 行)", name, sheet_data.len());
    }

    // ── 17. read_csv_bytes 显式读取 ──────────────────────────
    println!("\n▶ 17. read_csv_bytes 显式读取");
    let csv_bytes = std::fs::read("data.csv").unwrap();
    let data = Excel::read_csv_bytes(&csv_bytes).unwrap();
    println!("  ✅ 从 CSV 字节读取了 {} 行", data.len());

    // ── 18. read_bytes_as 自动检测格式 ───────────────────────
    println!("\n▶ 18. read_bytes_as 自动检测（先 Excel 后 CSV）");
    let xlsx_bytes = std::fs::read("simple.xlsx").unwrap();
    let data = Excel::read_bytes_as(&xlsx_bytes, None, None).unwrap();
    println!("  ✅ XLSX 自动检测: {} 行", data.len());
    let csv_bytes2 = std::fs::read("data.csv").unwrap();
    let data = Excel::read_bytes_as(&csv_bytes2, Some(Format::Csv), None).unwrap();
    println!("  ✅ CSV 指定格式: {} 行", data.len());

    // ═══════════════════════════════════════════════════════════
    //  流式读取测试
    // ═══════════════════════════════════════════════════════════

    // ── 19. RowReader 流式读取 CSV ──────────────────────────
    println!("\n▶ 19. RowReader 流式读取 CSV");
    Excel::write_as(
        &[
            &["Name", "Age", "City"],
            &["Alice", "25", "Beijing"],
            &["Bob", "30", "Shanghai"],
        ],
        Format::Csv,
    )
    .and_then(|b| {
        std::fs::write("stream.csv", b).unwrap();
        Ok(())
    })
    .unwrap();
    let mut reader = RowReader::from_csv_path("stream.csv").unwrap();
    while let Some(row) = reader.next_row() {
        let row = row.unwrap();
        let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
        println!("  | {}", cells.join(" | "));
    }

    // ── 20. RowReader 带表头 ─────────────────────────────────
    println!("\n▶ 20. RowReader 带表头（headers()）");
    let mut reader = RowReader::from_csv_path("stream.csv").unwrap();
    let headers = reader.headers().unwrap();
    println!("  表头: {:?}", headers);
    let mut count = 0;
    while let Some(_row) = reader.next_row() {
        count += 1;
    }
    println!("  数据行: {} 行", count);

    // ── 21. RowReader Iterator trait ─────────────────────────
    println!("\n▶ 21. RowReader Iterator trait（for 循环）");
    let reader = RowReader::from_csv_path("stream.csv").unwrap();
    for (i, row) in reader.enumerate() {
        let row = row.unwrap();
        println!("  第 {} 行: {:?}", i, row);
    }

    // ── 22. RowReader 读取 Excel ─────────────────────────────
    println!("\n▶ 22. RowReader 流式读取 Excel");
    let mut reader = RowReader::from_xlsx_path("simple.xlsx", None).unwrap();
    println!("  总行数: {:?}", reader.total_rows());
    let headers = reader.headers().unwrap();
    println!("  表头: {:?}", headers);
    while let Some(row) = reader.next_row() {
        let row = row.unwrap();
        let cells: Vec<String> = row.iter().map(|c| c.to_string()).collect();
        println!("  | {}", cells.join(" | "));
    }

    // ── 23. RowReader from_path 自动识别 ─────────────────────
    println!("\n▶ 23. RowReader from_path 自动识别");
    let reader = RowReader::from_path("stream.csv", None).unwrap();
    let rows: Vec<Vec<CellValue>> = reader.collect::<Result<Vec<_>, _>>().unwrap();
    println!("  CSV 读取 {} 行", rows.len());
    let reader = RowReader::from_path("simple.xlsx", None).unwrap();
    let rows: Vec<Vec<CellValue>> = reader.collect::<Result<Vec<_>, _>>().unwrap();
    println!("  XLSX 读取 {} 行", rows.len());

    // ── 24. RowReader collect_remaining ──────────────────────
    println!("\n▶ 24. RowReader collect_remaining");
    let mut reader = RowReader::from_csv_path("stream.csv").unwrap();
    reader.headers(); // 跳过表头
    let remaining = reader.collect_remaining().unwrap();
    println!("  剩余数据: {} 行", remaining.len());

    // ═══════════════════════════════════════════════════════════
    //  流式写入测试
    // ═══════════════════════════════════════════════════════════

    // ── 25. RowWriter 流式写入 CSV 文件 ──────────────────────
    println!("\n▶ 25. RowWriter 流式写入 CSV（from_csv_path）");
    {
        let mut writer = RowWriter::from_csv_path("stream_out.csv").unwrap();
        writer.write_row_str(&["Name", "Age", "Score"]).unwrap();
        writer
            .write_row(&[
                CellValue::String("Alice".into()),
                CellValue::Int(25),
                CellValue::Float(95.5),
            ])
            .unwrap();
        writer
            .write_row(&[
                CellValue::String("Bob".into()),
                CellValue::Int(30),
                CellValue::Float(88.0),
            ])
            .unwrap();
        writer.finish().unwrap();
    }
    println!("  ✅ 已写入 stream_out.csv");
    let content = std::fs::read_to_string("stream_out.csv").unwrap();
    println!("  内容:\n{}", content);

    // ── 26. RowWriter 内存积累 → CSV 字节 ───────────────────
    println!("▶ 26. RowWriter 内存积累 → CSV 字节");
    let mut writer = RowWriter::new();
    writer.write_row_str(&["Product", "Price"]).unwrap();
    writer
        .write_row(&[CellValue::String("Phone".into()), CellValue::Float(3999.0)])
        .unwrap();
    writer
        .write_row(&[CellValue::String("Tablet".into()), CellValue::Float(2999.0)])
        .unwrap();
    let bytes = writer.finish_csv_bytes().unwrap();
    println!("  ✅ CSV 字节 ({} bytes)", bytes.len());
    println!("  内容:\n{}", String::from_utf8_lossy(&bytes));

    // ── 27. RowWriter 内存积累 → Excel 字节 ─────────────────
    println!("▶ 27. RowWriter 内存积累 → Excel 字节");
    let mut writer = RowWriter::new();
    writer.write_row_str(&["ID", "Name"]).unwrap();
    writer
        .write_row(&[CellValue::Int(1), CellValue::String("Alice".into())])
        .unwrap();
    writer
        .write_row(&[CellValue::Int(2), CellValue::String("Bob".into())])
        .unwrap();
    let bytes = writer.finish_xlsx_bytes(Some("Users")).unwrap();
    std::fs::write("stream.xlsx", &bytes).unwrap();
    println!("  ✅ Excel 字节 ({} bytes)", bytes.len());

    // ── 28. RowWriter finish_bytes 指定格式 ──────────────────
    println!("▶ 28. RowWriter finish_bytes 指定格式");
    let mut writer = RowWriter::new();
    writer.write_row_str(&["Lang", "Stars"]).unwrap();
    writer
        .write_row(&[CellValue::String("Rust".into()), CellValue::Int(5)])
        .unwrap();
    let csv_bytes = writer.finish_bytes(Format::Csv).unwrap();
    let xlsx_bytes = writer.finish_bytes(Format::Xlsx).unwrap();
    println!(
        "  CSV: {} bytes, XLSX: {} bytes",
        csv_bytes.len(),
        xlsx_bytes.len()
    );

    // ── 29. RowWriter finish_path 保存文件 ───────────────────
    println!("▶ 29. RowWriter finish_path 保存文件");
    let mut writer = RowWriter::new();
    writer.write_row_str(&["Key", "Value"]).unwrap();
    writer
        .write_row(&[
            CellValue::String("version".into()),
            CellValue::String("1.0".into()),
        ])
        .unwrap();
    writer.finish_path("saved.csv", Format::Csv).unwrap();
    writer.finish_path("saved.xlsx", Format::Xlsx).unwrap_or(());
    // 注意: finish_path 消耗 rows，第二次调用会得到空文件
    // 正确做法是分别创建 writer
    let mut writer2 = RowWriter::new();
    writer2.write_row_str(&["Key", "Value"]).unwrap();
    writer2
        .write_row(&[
            CellValue::String("version".into()),
            CellValue::String("1.0".into()),
        ])
        .unwrap();
    writer2.finish_xlsx_path("saved.xlsx", None).unwrap();
    println!("  ✅ 已保存 saved.csv 和 saved.xlsx");

    // ── 30. RowWriter len/is_empty ───────────────────────────
    println!("▶ 30. RowWriter len/is_empty");
    let mut writer = RowWriter::new();
    println!(
        "  初始: len={}, is_empty={}",
        writer.len(),
        writer.is_empty()
    );
    writer.write_row_str(&["A", "B"]).unwrap();
    writer.write_row_str(&["1", "2"]).unwrap();
    println!(
        "  写入2行后: len={}, is_empty={}",
        writer.len(),
        writer.is_empty()
    );

    // ── 清理 ─────────────────────────────────────────────────
    println!("\n▶ 清理临时文件");
    let _ = std::fs::remove_file("simple.xlsx");
    let _ = std::fs::remove_file("typed.xlsx");
    let _ = std::fs::remove_file("multi.xlsx");
    let _ = std::fs::remove_file("headers.xlsx");
    let _ = std::fs::remove_file("data.csv");
    let _ = std::fs::remove_file("products.csv");
    let _ = std::fs::remove_file("langs.csv");
    let _ = std::fs::remove_file("stream.csv");
    let _ = std::fs::remove_file("stream_out.csv");
    let _ = std::fs::remove_file("stream.xlsx");
    let _ = std::fs::remove_file("saved.csv");
    let _ = std::fs::remove_file("saved.xlsx");
    println!("  ✅ 已清理\n");

    println!("═══════════════════════════════════════");
    println!("  ✅ 所有测试通过！");
    println!("═══════════════════════════════════════");
}
