use afaster::file::{FileConfig, FileService};

fn main() {
    println!("═══════════════════════════════════════");
    println!("  本地文件服务示例");
    println!("═══════════════════════════════════════");

    // 创建文件服务
    let config = FileConfig {
        root: "/tmp/afaster-file-example".to_string(),
        max_size: 5 * 1024 * 1024, // 5MB
        allowed: vec!["jpg".into(), "png".into(), "txt".into(), "pdf".into()],
        url_prefix: "/files".to_string(),
    };

    let fs = FileService::new(&config).expect("Failed to create FileService");

    // ── 1. 上传文件 ──────────────────────────────────────────
    println!("\n▶ 1. 上传文件");

    // 上传文本文件
    let info = fs
        .upload("docs/readme.txt", b"Hello, AFaster File Service!")
        .unwrap();
    println!(
        "  ✅ 上传: {} ({} bytes, {})",
        info.path, info.size, info.mime
    );

    // 上传到子目录
    let info = fs.upload("avatars/user1.txt", b"avatar data here").unwrap();
    println!("  ✅ 上传: {} ({} bytes)", info.path, info.size);

    // 上传 PDF
    let info = fs
        .upload("reports/monthly.txt", b"Monthly report content")
        .unwrap();
    println!("  ✅ 上传: {} ({} bytes)", info.path, info.size);

    // ── 2. 文件类型校验 ──────────────────────────────────────
    println!("\n▶ 2. 文件类型校验");

    match fs.upload("malware.exe", b"bad") {
        Ok(_) => println!("  ❌ 不应该成功"),
        Err(e) => println!("  ✅ 拒绝: {}", e),
    }

    // ── 3. 文件大小限制 ──────────────────────────────────────
    println!("\n▶ 3. 文件大小限制");

    let big_data = vec![0u8; 10 * 1024 * 1024]; // 10MB
    match fs.upload("big.bin", &big_data) {
        Ok(_) => println!("  ❌ 不应该成功"),
        Err(e) => println!("  ✅ 拒绝: {}", e),
    }

    // ── 4. 路径安全 ──────────────────────────────────────────
    println!("\n▶ 4. 路径安全");

    match fs.upload("../../etc/passwd", b"hack") {
        Ok(_) => println!("  ❌ 不应该成功"),
        Err(e) => println!("  ✅ 拒绝: {}", e),
    }

    // ── 5. 下载文件 ──────────────────────────────────────────
    println!("\n▶ 5. 下载文件");

    let data = fs.download("docs/readme.txt").unwrap();
    let content = String::from_utf8_lossy(&data);
    println!("  ✅ 下载: {}", content);

    // ── 6. 文件信息 ──────────────────────────────────────────
    println!("\n▶ 6. 文件信息");

    let info = fs.info("docs/readme.txt").unwrap();
    println!("  ✅ 路径: {}", info.path);
    println!("     名称: {}", info.name);
    println!("     扩展名: {}", info.ext);
    println!("     大小: {} bytes", info.size);
    println!("     MIME: {}", info.mime);

    // ── 7. 目录列表 ──────────────────────────────────────────
    println!("\n▶ 7. 目录列表");

    let list = fs.list("").unwrap();
    println!("  ✅ 根目录 ({} 项):", list.entries.len());
    for entry in &list.entries {
        let type_str = if entry.is_dir { "📁" } else { "📄" };
        println!("     {} {} ({} bytes)", type_str, entry.path, entry.size);
    }

    // 列出子目录
    let list = fs.list("docs").unwrap();
    println!("  ✅ docs/ ({} 项):", list.entries.len());
    for entry in &list.entries {
        println!("     📄 {} ({} bytes)", entry.name, entry.size);
    }

    // ── 8. 文件 URL ──────────────────────────────────────────
    println!("\n▶ 8. 文件 URL");

    let url = fs.url("docs/readme.txt").unwrap();
    println!("  ✅ URL: {}", url);

    // ── 9. 文件存在检查 ──────────────────────────────────────
    println!("\n▶ 9. 文件存在检查");

    println!("  ✅ docs/readme.txt: {}", fs.exists("docs/readme.txt"));
    println!("  ✅ not_exist.txt: {}", fs.exists("not_exist.txt"));

    // ── 10. 删除文件 ─────────────────────────────────────────
    println!("\n▶ 10. 删除文件");

    fs.delete("reports/monthly.txt").unwrap();
    println!("  ✅ 已删除 reports/monthly.txt");
    println!("     存在: {}", fs.exists("reports/monthly.txt"));

    // ── 11. 删除目录 ─────────────────────────────────────────
    println!("\n▶ 11. 删除目录");

    fs.delete("reports").unwrap();
    println!("  ✅ 已删除 reports/ 目录");

    // ── 清理 ─────────────────────────────────────────────────
    println!("\n▶ 清理");
    fs.delete("").unwrap();
    println!("  ✅ 已清理所有文件");

    println!("\n═══════════════════════════════════════");
    println!("  ✅ 所有测试通过！");
    println!("═══════════════════════════════════════");
}
