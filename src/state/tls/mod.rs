//! # TLS 模块
//!
//! 提供 TLS 配置管理与证书运行时重载功能。
//!
//! ## 配置
//!
//! 从 `config.toml` 的 `[tls]` 节反序列化：
//!
//! ```toml
//! [tls]
//! port = 443
//! cert_path = "/etc/ssl/cert.pem"
//! key_path = "/etc/ssl/key.pem"
//! ```
//!
//! ## 证书热重载
//!
//! 通过 `tokio::sync::broadcast` 通道向 afast 的 HTTPS 服务发送重载信号，
//! 实现不停机自动更新证书。
//!
//! ```ignore
//! AFaster::new("config.toml".into()).await?
//!     .with_acme(|acme| {
//!         acme.with_on_cert_obtained(|(state, _event)| async move {
//!             state.tls.reload();  // 通知 HTTPS 服务重载证书
//!             Ok(())
//!         })
//!     })
//!     .run().await;
//! ```

use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

// ═══════════════════════════════════════════════════════════════
//  TLS 配置
// ═══════════════════════════════════════════════════════════════

fn default_tls_port() -> u16 {
    443
}

/// TLS / HTTPS 配置
///
/// 从 config.toml 的 `[tls]` 节反序列化。
#[derive(Clone, Deserialize)]
pub struct TlsConfig {
    /// HTTPS 监听端口（默认 443）
    #[serde(default = "default_tls_port")]
    pub port: u16,
    /// PEM 证书链文件路径
    pub cert_path: String,
    /// PEM 私钥文件路径
    pub key_path: String,
}

// ═══════════════════════════════════════════════════════════════
//  TLS 重载通道
// ═══════════════════════════════════════════════════════════════

/// TLS 证书重载消息类型
///
/// - `None`：使用原始路径重新读取证书文件
/// - `Some(TlsReloadMessage)`：使用新的证书路径
pub type ReloadMessage = Option<afast::TlsReloadMessage>;

/// TLS 模块状态
///
/// 管理 TLS 配置与证书热重载通道。
/// 存储在 `AppState` 中，供 ACME 回调等模块触发证书重载。
///
/// 内部持有 `broadcast::Receiver`，在 `run()` 时取出传给 afast HTTPS 服务。
/// 取出后 `reload()` 方法仍可正常发送重载信号。
#[derive(Clone)]
pub struct Tls {
    /// TLS 配置
    pub config: TlsConfig,
    reload_tx: broadcast::Sender<ReloadMessage>,
    reload_rx: Arc<Mutex<Option<broadcast::Receiver<ReloadMessage>>>>,
}

impl Tls {
    /// 创建新的 TLS 模块
    pub fn new(config: TlsConfig) -> Self {
        let (tx, rx) = broadcast::channel(16);
        Self {
            config,
            reload_tx: tx,
            reload_rx: Arc::new(Mutex::new(Some(rx))),
        }
    }

    pub fn from_table(table: &toml::Table) -> crate::Result<Self> {
        let config: TlsConfig = crate::state::extract(table, "tls")?;
        Ok(Self::new(config))
    }

    /// 取出 Receiver（仅可调用一次）
    ///
    /// 返回的 Receiver 传给 `AFast::https()` 的 `reload_rx` 参数。
    /// 取出后再次调用返回 `None`。
    pub async fn take_receiver(&self) -> Option<broadcast::Receiver<ReloadMessage>> {
        self.reload_rx.lock().await.take()
    }

    /// 使用原始路径重载证书（重新读取文件）
    ///
    /// 适用于 ACME 续期后证书文件已更新，但路径不变的场景。
    pub fn reload(&self) {
        let _ = self.reload_tx.send(None);
    }

    /// 使用新路径重载证书
    ///
    /// 适用于证书文件路径发生变化的场景。
    pub fn reload_with(&self, cert_path: &str, key_path: &str) {
        let _ = self.reload_tx.send(Some(afast::TlsReloadMessage {
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
        }));
    }
}
