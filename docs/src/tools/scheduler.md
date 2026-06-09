# Scheduler

Feature: `scheduler` | 依赖: `cron`

## 简介

定时任务调度器，支持 Cron 表达式。每个任务独立 [`tokio::spawn`](https://docs.rs/tokio) 运行，互不影响，不阻塞接口处理。

## 配置

无需配置文件，所有任务在代码中注册：

```rust
use afaster::{AppState, Overlap};

let state = AppState::new("config.toml".to_string()).await?;
let scheduler = &state.scheduler;
```

### 通过 AFaster 构建器注册

推荐使用 `with_scheduler` 或 `schedulers` 方法在构建器中注册定时任务：

```rust
use afaster::AFaster;

AFaster::new("config.toml".into()).await?
    .with_scheduler(|s| {
        s.add_task("cleanup", "*/5 * * * *", |state, name, times| async move {
            println!("[{}] 第 {} 次执行", name, times);
        }).unwrap();
        s
    })
    .run()
    .await;
```

### 批量注册定时任务

```rust
AFaster::new("config.toml".into()).await?
    .schedulers(vec![
        Box::new(|s| {
            s.add_task("cleanup", "*/5 * * * *", |state, name, times| async move {
                println!("[{}] 第 {} 次执行", name, times);
            }).unwrap();
            s
        }),
        Box::new(|s| {
            s.add_times("import", "* * * * *", 10, |state, name, times| async move {
                println!("{}: {}/10", name, times);
            }).unwrap();
            s
        }),
        Box::new(|s| {
            s.add_with_overlap("sync", "*/1 * * * *", Overlap::Concurrent, |state, name, times| async move {
                // ...
            }).unwrap();
            s
        }),
    ])
    .run()
    .await;
```

## API

### 添加任务

```rust
// 最简：Skip + 永不删除
scheduler.add_task("cleanup", "*/5 * * * *", |state, name, times| async move {
    println!("[{}] 第 {} 次执行", name, times);
}).unwrap();

// 执行 10 次后自动删除
scheduler.add_times("import", "* * * * *", 10, |state, name, times| async move {
    println!("{}: {}/10", name, times);
}).unwrap();

// 指定重叠策略：永不删除
scheduler.add_with_overlap("sync", "*/1 * * * *", Overlap::Concurrent, |state, name, times| async move {
    // ...
}).unwrap();

// 全自定义
scheduler.add("report", "0 9 * * *", Overlap::Queue, Some(365), |state, name, times| async move {
    // ...
}).unwrap();
```

### 任务函数参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `state` | `AppState` | 共享状态，访问 redis/db/push 等 |
| `name` | `String` | 任务名，用于 `remove(&name)` / `pause(&name)` |
| `times` | `usize` | 当前第几次执行（从 1 开始） |

### 操作任务

```rust
// 暂停（触发时间到了不执行，等待 resume）
scheduler.pause("sync").await;

// 恢复
scheduler.resume("sync").await;

// 移除（停止循环，从列表清除）
scheduler.remove("sync").await;

// 查询
scheduler.contains("sync").await;  // bool
scheduler.len().await;             // usize
scheduler.is_empty().await;        // bool
```

### 任务内自删除

```rust
scheduler.add_task("sync", "* * * * *", |state, name, _times| async move {
    if should_stop(&state).await {
        state.scheduler.remove(&name).await;  // 自己删自己
    }
}).unwrap();
```

## Overlap 重叠策略

```rust
pub enum Overlap {
    Skip,       // 上次还在跑 → 跳过本次（默认，推荐）
    Queue,      // 上次还在跑 → 等完再执行
    Concurrent, // 上次还在跑 → 再开一个，允许并行
}
```

### Skip（默认）

```
任务A: cron="*/5 * * * *"，实际运行8分钟

0min         5min              10min
|───A运行───────|
         (跳过)  |───A运行───────|
                            (跳过)
```

### Queue

```
0min         5min         8min      13min
|───A运行───────|
                 |等|───A运行───────|
                                |等|───A运行
```

### Concurrent

```
0min         5min              10min
|───A运行───────|
         |───B运行───────|
                  |───C运行───────|
```

## Cron 表达式

5 位格式（分 时 日 月 周）：

```
┌───────────── 分钟 (0-59)
│ ┌─────────── 小时 (0-23)
│ │ ┌───────── 日 (1-31)
│ │ │ ┌─────── 月 (1-12)
│ │ │ │ ┌───── 星期 (0-6, 日=0)
│ │ │ │ │
* * * * *
```

| 表达式 | 说明 |
|--------|------|
| `* * * * *` | 每分钟 |
| `*/5 * * * *` | 每 5 分钟 |
| `0 * * * *` | 每小时整点 |
| `0 9 * * *` | 每天 9:00 |
| `0 9 * * 1-5` | 工作日 9:00 |
| `0 0 1 * *` | 每月 1 号 0:00 |
| `30 4 * * 0` | 每周日 4:30 |

## 错误码

| 错误码 | 说明 |
|--------|------|
| 51501 | Cron 表达式解析失败 |

## 注意事项

- 每个任务用 `tokio::spawn` 独立运行，不阻塞接口 handler
- 任务间完全隔离，互不影响
- 精度为秒级，取决于 tokio runtime 调度
- 任务函数通过 `state.scheduler.remove(&name)` 可安全自删除（当前执行会完整跑完）
