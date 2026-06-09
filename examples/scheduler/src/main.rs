use afaster::AppState;
use afaster::scheduler::Overlap;

#[tokio::main]
async fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              Scheduler 定时任务演示                      ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // ── 初始化 AppState ──────────────────────────────────────
    let state = AppState::new("examples/scheduler/config.toml".to_string())
        .await
        .expect("初始化 AppState 失败");

    let scheduler = &state.scheduler;

    // ════════════════════════════════════════════════════════════
    //  1. 最简任务：每分钟执行，永不删除
    // ════════════════════════════════════════════════════════════
    println!("▶ 添加任务: heartbeat (每分钟)");
    scheduler
        .add_task("heartbeat", "* * * * *", |_state, name, times| async move {
            println!("  💓 [{}] 第 {} 次心跳", name, times);
        })
        .unwrap();

    // ════════════════════════════════════════════════════════════
    //  2. 限时任务：每分钟执行，执行 3 次后自动删除
    // ════════════════════════════════════════════════════════════
    println!("▶ 添加任务: import (每分钟, 3次后自动删除)");
    scheduler
        .add_times("import", "* * * * *", 3, |_state, name, times| async move {
            println!("  📦 [{}] 导入数据 {}/3", name, times);
        })
        .unwrap();

    // ════════════════════════════════════════════════════════════
    //  3. 允许并行任务：每分钟执行，上次没跑完也再开一个
    // ════════════════════════════════════════════════════════════
    println!("▶ 添加任务: sync (每分钟, 允许并行)");
    scheduler
        .add_with_overlap(
            "sync",
            "* * * * *",
            Overlap::Concurrent,
            |_state, name, times| async move {
                println!("  🔄 [{}] 同步开始 #{}", name, times);
                // 模拟耗时操作
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                println!("  🔄 [{}] 同步完成 #{}", name, times);
            },
        )
        .unwrap();

    // ════════════════════════════════════════════════════════════
    //  4. 全参数任务：排队执行，执行 5 次后自动删除
    // ════════════════════════════════════════════════════════════
    println!("▶ 添加任务: report (每分钟, 排队, 5次后删除)");
    scheduler
        .add(
            "report",
            "* * * * *",
            Overlap::Queue,
            Some(5),
            |_state, name, times| async move {
                println!("  📊 [{}] 生成报表 #{}", name, times);
            },
        )
        .unwrap();

    // ════════════════════════════════════════════════════════════
    //  演示暂停/恢复
    // ════════════════════════════════════════════════════════════
    println!("\n当前任务数量: {}", scheduler.len().await);
    println!("任务列表: heartbeat, import, sync, report\n");

    // 暂停 sync
    scheduler.pause("sync").await;
    println!("⏸ 已暂停 sync");

    // 10 秒后恢复
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    scheduler.resume("sync").await;
    println!("▶ 已恢复 sync\n");

    // ════════════════════════════════════════════════════════════
    //  保持运行
    // ════════════════════════════════════════════════════════════
    println!("调度器运行中，按 Ctrl+C 退出...\n");

    // 等待 Ctrl+C
    tokio::signal::ctrl_c().await.ok();

    // 清理
    println!("\n正在停止所有任务...");
    scheduler.remove("heartbeat").await;
    scheduler.remove("sync").await;
    // import 和 report 会自动删除
    println!("✅ 调度器已停止");
}
