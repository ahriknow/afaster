use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use cron::Schedule;
use tokio::sync::{Mutex, watch};

// ═══════════════════════════════════════════════════════════════
//  定时任务调度器
// ═══════════════════════════════════════════════════════════════

/// 异步任务函数类型
///
/// 参数: `(state, name, times)`
/// - `state`: AppState 共享状态
/// - `name`: 任务名
/// - `times`: 当前第几次执行（从 1 开始）
type TaskFn = Arc<
    dyn Fn(super::AppState, String, usize) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// 重叠策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    /// 上一次还在跑 → 跳过本次（默认，推荐）
    Skip,
    /// 上一次还在跑 → 等它跑完再执行本次
    Queue,
    /// 上一次还在跑 → 再开一个，允许并行
    Concurrent,
}

/// 任务控制信号
struct TaskControl {
    /// 是否暂停
    paused: AtomicBool,
    /// 取消信号（发送 true 表示停止）
    cancel_tx: watch::Sender<bool>,
}

/// 任务条目
#[allow(dead_code)]
struct TaskEntry {
    /// 控制信号
    control: Arc<TaskControl>,
    /// 当前正在运行的任务数量
    running: Arc<AtomicUsize>,
    /// 已执行次数
    times: Arc<AtomicUsize>,
}

/// 定时任务调度器
///
/// 支持 Cron 表达式调度，每个任务独立 [`tokio::spawn`] 运行，互不影响。
/// 任务函数接收 `(state, name, times)` 三个参数。
///
/// ```no_run
/// use afaster::{Scheduler, Overlap};
///
/// let scheduler = Scheduler::new();
///
/// // 最简用法
/// scheduler.add_task("cleanup", "*/5 * * * *", |state, name, times| async move {
///     println!("[{}] 第 {} 次执行", name, times);
/// }).unwrap();
///
/// // 执行 10 次后自动删除
/// scheduler.add_times("import", "* * * * *", 10, |state, name, times| async move {
///     println!("{}: {}/10", name, times);
/// }).unwrap();
///
/// // 允许并行
/// scheduler.add_with_overlap("sync", "*/1 * * * *", Overlap::Concurrent, |state, name, times| async move {
///     // ...
/// }).unwrap();
/// ```
#[derive(Clone)]
pub struct Scheduler {
    tasks: Arc<Mutex<HashMap<String, TaskEntry>>>,
    state: Arc<Mutex<Option<super::AppState>>>,
}

impl Scheduler {
    /// 创建新的调度器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            state: Arc::new(Mutex::new(None)),
        }
    }

    /// 初始化 state（内部调用，AppState 构造时自动执行）
    pub async fn init_state(&self, state: super::AppState) {
        let mut s = self.state.lock().await;
        *s = Some(state);
    }

    // ── 添加任务 ─────────────────────────────────────────────

    /// 添加任务（完整参数）
    ///
    /// - `name`: 任务唯一标识
    /// - `cron_expr`: Cron 表达式（5 位: 分 时 日 月 周）
    /// - `overlap`: 重叠策略
    /// - `auto_remove`: 执行 N 次后自动移除，`None` = 永不自动移除
    /// - `task`: 异步任务函数 `|state, name, times| async move { ... }`
    pub fn add<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        overlap: Overlap,
        auto_remove: Option<usize>,
        task: F,
    ) -> crate::Result<()>
    where
        F: Fn(super::AppState, String, usize) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let schedule = Schedule::from_str(cron_expr).map_err(|e| {
            crate::Error::custom(
                51501,
                format!("Invalid cron expression '{}': {}", cron_expr, e),
            )
        })?;

        let task_fn: TaskFn =
            Arc::new(move |state, name, times| Box::pin(task(state, name, times)));

        self.spawn(name, schedule, overlap, auto_remove, task_fn)
    }

    /// 添加任务（最简：Skip + 永不删除）
    pub fn add_task<F, Fut>(&self, name: &str, cron_expr: &str, task: F) -> crate::Result<()>
    where
        F: Fn(super::AppState, String, usize) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.add(name, cron_expr, Overlap::Skip, None, task)
    }

    /// 添加任务（指定执行次数后自动删除：Skip）
    pub fn add_times<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        times: usize,
        task: F,
    ) -> crate::Result<()>
    where
        F: Fn(super::AppState, String, usize) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.add(name, cron_expr, Overlap::Skip, Some(times), task)
    }

    /// 添加任务（指定重叠策略：永不删除）
    pub fn add_with_overlap<F, Fut>(
        &self,
        name: &str,
        cron_expr: &str,
        overlap: Overlap,
        task: F,
    ) -> crate::Result<()>
    where
        F: Fn(super::AppState, String, usize) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.add(name, cron_expr, overlap, None, task)
    }

    // ── 操作方法 ─────────────────────────────────────────────

    /// 暂停任务（触发时间到了不执行，等待 resume）
    pub async fn pause(&self, name: &str) -> bool {
        let tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.get(name) {
            entry.control.paused.store(true, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// 恢复任务
    pub async fn resume(&self, name: &str) -> bool {
        let tasks = self.tasks.lock().await;
        if let Some(entry) = tasks.get(name) {
            entry.control.paused.store(false, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// 移除任务（停止循环，从列表清除）
    pub async fn remove(&self, name: &str) -> bool {
        let entry = {
            let mut tasks = self.tasks.lock().await;
            tasks.remove(name)
        };
        if let Some(entry) = entry {
            // 发送取消信号
            let _ = entry.control.cancel_tx.send(true);
            true
        } else {
            false
        }
    }

    /// 是否存在
    pub async fn contains(&self, name: &str) -> bool {
        self.tasks.lock().await.contains_key(name)
    }

    /// 任务数量
    pub async fn len(&self) -> usize {
        self.tasks.lock().await.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.tasks.lock().await.is_empty()
    }

    // ── 内部方法 ─────────────────────────────────────────────

    /// 启动任务循环
    fn spawn(
        &self,
        name: &str,
        schedule: Schedule,
        overlap: Overlap,
        auto_remove: Option<usize>,
        task_fn: TaskFn,
    ) -> crate::Result<()> {
        let name = name.to_string();
        let (cancel_tx, mut cancel_rx) = watch::channel(false);
        let control = Arc::new(TaskControl {
            paused: AtomicBool::new(false),
            cancel_tx,
        });
        let running = Arc::new(AtomicUsize::new(0));
        let times = Arc::new(AtomicUsize::new(0));

        let entry = TaskEntry {
            control: control.clone(),
            running: running.clone(),
            times: times.clone(),
        };

        let tasks = self.tasks.clone();
        let state_ref = self.state.clone();
        let task_name = name.clone();

        // 在当前 tokio runtime 中 spawn
        tokio::spawn(async move {
            // 注册任务条目
            {
                let mut tasks = tasks.lock().await;
                tasks.insert(task_name.clone(), entry);
            }

            // 获取 state
            let state = {
                let s = state_ref.lock().await;
                match s.as_ref() {
                    Some(s) => s.clone(),
                    None => return,
                }
            };

            while let Some(next) = schedule.upcoming(chrono::Utc).next() {
                let delay = (next - chrono::Utc::now()).to_std().unwrap_or_default();

                // 等待到触发时间，同时监听取消信号
                tokio::select! {
                    _ = tokio::time::sleep(delay) => {}
                    _ = cancel_rx.changed() => {
                        if *cancel_rx.borrow() {
                            break;
                        }
                    }
                }

                // 再次检查取消信号
                if *cancel_rx.borrow() {
                    break;
                }

                // 暂停中 → 跳过
                if control.paused.load(Ordering::Acquire) {
                    continue;
                }

                // 重叠策略
                match overlap {
                    Overlap::Skip => {
                        if running.load(Ordering::Acquire) > 0 {
                            continue;
                        }
                    }
                    Overlap::Queue => {
                        // 等待上一次完成
                        while running.load(Ordering::Acquire) > 0 {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            if *cancel_rx.borrow() {
                                break;
                            }
                        }
                        if *cancel_rx.borrow() {
                            break;
                        }
                    }
                    Overlap::Concurrent => {
                        // 直接允许并行
                    }
                }

                // 检查是否达到自动删除次数
                let current_times = times.load(Ordering::Acquire);
                if auto_remove.is_some_and(|limit| current_times >= limit) {
                    break;
                }

                // 递增执行次数
                let exec_times = times.fetch_add(1, Ordering::AcqRel) + 1;

                // spawn 任务执行
                let running = running.clone();
                let task_fn = task_fn.clone();
                let name_clone = task_name.clone();
                let cancel_tx_clone = control.cancel_tx.clone();
                let tasks_clone = tasks.clone();
                let state_clone = state.clone();

                running.fetch_add(1, Ordering::AcqRel);

                tokio::spawn(async move {
                    task_fn(state_clone, name_clone.clone(), exec_times).await;
                    running.fetch_sub(1, Ordering::AcqRel);

                    // 检查是否需要自动删除
                    if auto_remove.is_some_and(|limit| exec_times >= limit) {
                        let _ = cancel_tx_clone.send(true);
                        let mut tasks = tasks_clone.lock().await;
                        tasks.remove(&name_clone);
                    }
                });
            }

            // 循环结束后清理
            let mut tasks = tasks.lock().await;
            tasks.remove(&task_name);
        });

        Ok(())
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster 定时任务配置扩展
pub trait AFasterSchedulerExt {
    /// 链式注册定时任务
    fn with_scheduler(self, f: impl FnOnce(Scheduler) -> Scheduler) -> Self;

    /// 批量注册定时任务
    fn schedulers(self, fs: Vec<Box<dyn FnOnce(Scheduler) -> Scheduler>>) -> Self;
}

impl AFasterSchedulerExt for crate::AFaster {
    fn with_scheduler(mut self, f: impl FnOnce(Scheduler) -> Scheduler) -> Self {
        self.state.scheduler = f(self.state.scheduler);
        self
    }

    fn schedulers(mut self, fs: Vec<Box<dyn FnOnce(Scheduler) -> Scheduler>>) -> Self {
        for f in fs {
            self.state.scheduler = f(self.state.scheduler);
        }
        self
    }
}
