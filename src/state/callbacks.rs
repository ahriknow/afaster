//! # 回调注册系统
//!
//! 提供通用异步回调类型。
//!
//! 各平台的具体回调类型定义在对应的 state 子模块中。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ═══════════════════════════════════════════════════════════════
//  通用异步回调类型
// ═══════════════════════════════════════════════════════════════

/// 通用异步回调类型别名
///
/// - `In`: 回调输入类型（通常为元组）
/// - `Out`: 回调输出类型
#[allow(dead_code)]
pub type AsyncCallback<In, Out> =
    Arc<dyn Fn(In) -> Pin<Box<dyn Future<Output = Out> + Send>> + Send + Sync>;

// ═══════════════════════════════════════════════════════════════
//  回调自动装箱 trait
// ═══════════════════════════════════════════════════════════════

/// 将闭包转换为 `AsyncCallback`
///
/// 自动将 `impl Fn(...) -> impl Future` 装箱为 `Arc<dyn Fn(...) -> Pin<Box<dyn Future>> + Send + Sync>`。
/// 用户无需手动调用 `Arc::new` 或 `Box::pin`。
#[allow(dead_code)]
pub trait IntoCallback<In, Out>: Send + Sync + 'static {
    fn into_callback(self) -> AsyncCallback<In, Out>;
}

impl<F, Fut, In, Out> IntoCallback<In, Out> for F
where
    In: 'static,
    Out: 'static,
    F: Fn(In) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Out> + Send + 'static,
{
    fn into_callback(self) -> AsyncCallback<In, Out> {
        Arc::new(move |args| Box::pin(self(args)))
    }
}
