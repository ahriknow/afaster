use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::state::callbacks::AsyncCallback;

// ═══════════════════════════════════════════════════════════════
//  ACME 配置
// ═══════════════════════════════════════════════════════════════

fn default_cache_dir() -> String {
    "./acme_cache".to_string()
}

fn default_allow_non_80() -> bool {
    false
}

fn default_renewal_days() -> u32 {
    10
}

/// ACME 自动证书配置
///
/// 从 config.toml 的 `[acme]` 节反序列化。
///
/// 使用 HTTP-01 验证方式，通过 afast HTTP 服务的路由响应 Let's Encrypt 验证请求。
/// ACME 模块自身不启动任何网络监听，所有验证请求由 afast 的 HTTP 服务处理。
/// HTTPS 端口使用 `[tls].port` 配置。
#[derive(Debug, Clone, Deserialize)]
pub struct AcmeConfig {
    /// 申请证书的域名列表（多域名将生成一个 SAN 证书）
    pub domains: Vec<String>,
    /// 联系邮箱（可选，用于证书到期提醒）
    pub contact: Option<String>,
    /// 证书缓存目录（可选，默认 ./acme_cache）
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// 是否使用 Let's Encrypt 测试环境（可选，默认 false）
    #[serde(default)]
    pub staging: bool,
    /// 证书到期前多少天自动续期（可选，默认 10）
    #[serde(default = "default_renewal_days")]
    pub renewal_days: u32,
    /// 是否允许非 80 端口（可选，默认 false）
    ///
    /// HTTP-01 验证要求 Let's Encrypt 访问 80 端口。
    /// 设为 false 时，如果 backend.port 不是 80 会报错。
    /// 设为 true 时，用户需自行配置反向代理将 80 端口转发到服务端口。
    #[serde(default = "default_allow_non_80")]
    pub allow_non_80: bool,
}

// ═══════════════════════════════════════════════════════════════
//  证书事件与回调类型
// ═══════════════════════════════════════════════════════════════

/// 证书事件信息
///
/// 当 ACME 成功获取或续期证书时，通过回调传递给用户。
/// 用户可在回调中调用 `state.tls.reload()` 触发 HTTPS 服务热重载证书。
#[derive(Clone)]
pub struct CertEvent {
    /// 证书链文件路径
    pub cert_path: PathBuf,
    /// 私钥文件路径
    pub key_path: PathBuf,
    /// 证书覆盖的域名列表
    pub domains: Vec<String>,
}

/// 证书错误事件信息
///
/// 当 ACME 证书申请或续期失败时，通过回调传递给用户。
#[derive(Clone)]
pub struct CertErrorEvent {
    /// 错误信息
    pub error: String,
    /// 证书覆盖的域名列表
    pub domains: Vec<String>,
}

/// 证书获取成功回调函数类型
///
/// 首次申请证书成功时调用。
pub type CertObtainedCallbackFn = AsyncCallback<(crate::AppState, CertEvent), crate::Result<()>>;

/// 证书续期成功回调函数类型
///
/// 证书自动续期成功时调用。
pub type CertRenewedCallbackFn = AsyncCallback<(crate::AppState, CertEvent), crate::Result<()>>;

/// 证书失败回调函数类型
///
/// 证书申请或续期失败时调用。
pub type CertFailedCallbackFn = AsyncCallback<(crate::AppState, CertErrorEvent), crate::Result<()>>;

/// ACME 运行时状态（不序列化）
#[derive(Clone, Default)]
pub struct AcmeRuntime {
    pub(crate) on_cert_obtained: Option<CertObtainedCallbackFn>,
    pub(crate) on_cert_renewed: Option<CertRenewedCallbackFn>,
    pub(crate) on_cert_failed: Option<CertFailedCallbackFn>,
}

// ═══════════════════════════════════════════════════════════════
//  Challenge 存储
// ═══════════════════════════════════════════════════════════════

/// ACME HTTP-01 挑战令牌存储
///
/// 在 ACME 证书申请过程中，`obtain_certificates` 将挑战 token 和 key_authorization
/// 写入此存储，afast HTTP 服务的 `/.well-known/acme-challenge/{token}` 路由
/// 从中读取并返回给 Let's Encrypt 验证服务器。
pub type ChallengeStore = Arc<RwLock<HashMap<String, String>>>;

// ═══════════════════════════════════════════════════════════════
//  ACME 状态
// ═══════════════════════════════════════════════════════════════

/// ACME 证书管理状态
///
/// 管理 Let's Encrypt 证书的申请、续期和缓存。
/// 证书申请和续期在后台异步进行，验证请求通过 afast HTTP 路由处理。
///
/// 支持通过 `with_on_cert_obtained` 和 `with_on_cert_renewed` 注册回调，
/// 当证书获取或续期成功时自动调用。
///
/// ## 示例
///
/// ```ignore
/// AFaster::new("config.toml".into()).await?
///     .with_acme(|acme| {
///         acme.with_on_cert_obtained(|(state, _event)| async move {
///             state.tls.reload();  // 触发 HTTPS 热重载
///             Ok(())
///         })
///         .with_on_cert_renewed(|(state, _event)| async move {
///             state.tls.reload();
///             Ok(())
///         })
///     })
///     .run().await;
/// ```
#[derive(Clone)]
pub struct AcmeState {
    /// ACME 配置
    pub config: AcmeConfig,
    /// HTTP-01 挑战令牌存储
    pub challenge_store: ChallengeStore,
    /// 缓存目录路径
    cache_path: PathBuf,
    /// 运行时回调
    runtime: AcmeRuntime,
}

impl AcmeState {
    /// 创建新的 ACME 状态
    pub fn new(config: AcmeConfig) -> Self {
        let cache_path = PathBuf::from(&config.cache_dir);
        Self {
            config,
            challenge_store: Arc::new(RwLock::new(HashMap::new())),
            cache_path,
            runtime: AcmeRuntime::default(),
        }
    }

    pub fn from_table(table: &toml::Table, port: u16) -> crate::Result<Self> {
        let config: AcmeConfig = crate::state::extract(table, "acme")?;
        if !config.allow_non_80 && port != 80 {
            return Err(crate::Error::custom(
                50001,
                format!(
                    "ACME HTTP-01 验证要求 80 端口, 但 backend.port = {}. \
                     设置 [acme].allow_non_80 = true 可跳过此检查 (需自行配置反向代理)",
                    port
                ),
            ));
        }
        Ok(Self::new(config))
    }

    /// 注册证书获取成功回调
    ///
    /// 首次申请证书成功时调用。可在回调中调用 `state.tls.reload()` 触发热重载。
    pub fn with_on_cert_obtained(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<(crate::AppState, CertEvent), crate::Result<()>>,
    ) -> Self {
        self.runtime.on_cert_obtained = Some(cb.into_callback());
        self
    }

    /// 注册证书续期成功回调
    ///
    /// 证书自动续期成功时调用。可在回调中调用 `state.tls.reload()` 触发热重载。
    pub fn with_on_cert_renewed(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<(crate::AppState, CertEvent), crate::Result<()>>,
    ) -> Self {
        self.runtime.on_cert_renewed = Some(cb.into_callback());
        self
    }

    /// 注册证书失败回调
    ///
    /// 证书申请或续期失败时调用。可用于发送告警通知。
    pub fn with_on_cert_failed(
        mut self,
        cb: impl crate::state::callbacks::IntoCallback<
            (crate::AppState, CertErrorEvent),
            crate::Result<()>,
        >,
    ) -> Self {
        self.runtime.on_cert_failed = Some(cb.into_callback());
        self
    }

    /// 检查缓存中是否存在证书文件
    pub fn has_cached_certs(&self) -> bool {
        self.cert_path().exists() && self.key_path().exists()
    }

    /// 获取证书链文件路径
    pub fn cert_path(&self) -> PathBuf {
        self.cache_path.join("fullchain.pem")
    }

    /// 获取私钥文件路径
    pub fn key_path(&self) -> PathBuf {
        self.cache_path.join("privkey.pem")
    }

    /// 获取或申请 TLS 证书
    ///
    /// - 如果缓存中已有有效证书则直接返回路径
    /// - 否则通过 Let's Encrypt HTTP-01 验证自动申请
    /// - 申请过程中会将挑战 token 写入 `challenge_store`，供 afast HTTP 路由使用
    ///
    /// 返回 `(cert_pem_path, key_pem_path)`
    pub async fn obtain_certificates(&self) -> crate::Result<(PathBuf, PathBuf)> {
        let cert_path = self.cert_path();
        let key_path = self.key_path();

        // 缓存命中直接返回
        if cert_path.exists() && key_path.exists() {
            #[cfg(feature = "log")]
            tracing::info!("ACME: 使用缓存证书 {:?} / {:?}", cert_path, key_path);
            return Ok((cert_path, key_path));
        }

        #[cfg(feature = "log")]
        tracing::info!(
            "ACME: 为 {:?} 申请证书 ({}环境)...",
            self.config.domains,
            if self.config.staging {
                "测试"
            } else {
                "生产"
            }
        );

        // 确保缓存目录存在
        tokio::fs::create_dir_all(&self.cache_path)
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 创建缓存目录失败: {}", e)))?;

        // ── 1. 创建 TLS 客户端配置 ──
        use rustls::RootCertStore;
        use rustls_acme::futures_rustls::rustls::{self, ClientConfig};

        let _ = rustls::crypto::ring::default_provider().install_default();

        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let client_config = std::sync::Arc::new(
            ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth(),
        );

        #[cfg(feature = "log")]
        tracing::info!("ACME: [1/7] TLS 客户端配置完成");

        // ── 2. 发现 ACME 目录 ──
        let dir_url = if self.config.staging {
            rustls_acme::acme::LETS_ENCRYPT_STAGING_DIRECTORY
        } else {
            rustls_acme::acme::LETS_ENCRYPT_PRODUCTION_DIRECTORY
        };

        let directory = rustls_acme::acme::Directory::discover(&client_config, dir_url)
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 发现目录失败: {}", e)))?;

        #[cfg(feature = "log")]
        tracing::info!("ACME: [2/7] ACME 目录发现成功");

        // ── 3. 创建/获取账户 ──
        let contacts: Vec<&str> = self.config.contact.iter().map(|s| s.as_str()).collect();
        let account =
            rustls_acme::acme::Account::create(&client_config, directory, contacts.iter())
                .await
                .map_err(|e| crate::Error::custom(50020, format!("ACME: 创建账户失败: {}", e)))?;

        #[cfg(feature = "log")]
        tracing::info!("ACME: [3/7] 账户创建成功");

        // ── 4. 创建证书订单 ──
        let domains: Vec<String> = self.config.domains.clone();
        let (order_url, mut order) = account
            .new_order(&client_config, domains)
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 创建订单失败: {}", e)))?;

        #[cfg(feature = "log")]
        tracing::info!("ACME: [4/7] 证书订单已创建, order_url={}", order_url);

        // ── 5. 获取授权、注册挑战、通知验证 ──
        let auth_urls = order.authorizations.clone();
        #[cfg(feature = "log")]
        tracing::info!("ACME: [5/7] 开始处理 {} 个域名授权", auth_urls.len());
        for (i, auth_url) in auth_urls.iter().enumerate() {
            let auth = account
                .auth(&client_config, auth_url)
                .await
                .map_err(|e| crate::Error::custom(50020, format!("ACME: 获取授权失败: {}", e)))?;

            #[cfg(feature = "log")]
            tracing::info!(
                "ACME:   域名 {}/{}: {} (status={:?})",
                i + 1,
                auth_urls.len(),
                match &auth.identifier {
                    rustls_acme::acme::Identifier::Dns(d) => d.clone(),
                },
                auth.status
            );

            let (challenge, key_auth) = account.http_01(&auth.challenges).map_err(|e| {
                crate::Error::custom(50020, format!("ACME: 该授权不支持 HTTP-01 验证: {}", e))
            })?;

            // 将挑战令牌写入共享存储，供 afast HTTP 路由响应验证请求
            let token = challenge.token.clone();

            #[cfg(feature = "log")]
            tracing::info!("ACME:   注册挑战 token={}", token);

            self.challenge_store.write().await.insert(token, key_auth);

            // 通知 ACME 服务器开始验证（会触发 Let's Encrypt 请求我们的 HTTP 路由）
            account
                .challenge(&client_config, &challenge.url)
                .await
                .map_err(|e| crate::Error::custom(50020, format!("ACME: 验证通知失败: {}", e)))?;

            #[cfg(feature = "log")]
            tracing::info!(
                "ACME:   域名 {} 已通知验证",
                match &auth.identifier {
                    rustls_acme::acme::Identifier::Dns(d) => d.clone(),
                }
            );
        }

        // ── 6. 等待所有域名验证完成 ──
        #[cfg(feature = "log")]
        tracing::info!("ACME: [6/8] 所有域名已通知验证, 等待 Let's Encrypt 验证...");

        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts > 150 {
                return Err(crate::Error::custom(50020, "ACME: 域名验证超时 (300秒)"));
            }

            order = account
                .order(&client_config, &order_url)
                .await
                .map_err(|e| crate::Error::custom(50020, format!("ACME: 查询订单失败: {}", e)))?;

            match &order.status {
                rustls_acme::acme::OrderStatus::Ready => break,
                rustls_acme::acme::OrderStatus::Invalid => {
                    #[cfg(feature = "log")]
                    tracing::error!("ACME:   订单状态: Invalid, 验证失败");
                    return Err(crate::Error::custom(50020, "ACME: 订单无效, 域名验证失败"));
                }
                _ => {
                    #[cfg(feature = "log")]
                    tracing::debug!(
                        "ACME:   订单状态: {:?}, 等待中... ({}/150)",
                        order.status,
                        attempts
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }

        #[cfg(feature = "log")]
        tracing::info!("ACME: [5/7] 域名验证通过!");

        // ── 6. 生成 CSR 并完成订单 ──
        #[cfg(feature = "log")]
        tracing::info!("ACME: [6/7] 生成 CSR...");

        let key_pair = rcgen::KeyPair::generate()
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 生成密钥对失败: {}", e)))?;

        let mut params = rcgen::CertificateParams::new(self.config.domains.clone())
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 创建证书参数失败: {}", e)))?;

        // 设置 CN 为第一个域名（避免 rcgen 默认的 "rcgen self signed cert"）
        let mut dn = rcgen::DistinguishedName::new();
        dn.push(rcgen::DnType::CommonName, &self.config.domains[0]);
        params.distinguished_name = dn;

        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 生成 CSR 失败: {}", e)))?;

        account
            .finalize(&client_config, &order.finalize, csr.der())
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 完成订单失败: {}", e)))?;

        #[cfg(feature = "log")]
        tracing::info!("ACME: [6/7] CSR 已提交, 等待证书签发...");

        // 等待订单变为 Valid 状态 (最多 120 秒)
        let mut attempts = 0;
        let certificate_url = loop {
            attempts += 1;
            if attempts > 60 {
                return Err(crate::Error::custom(50020, "ACME: 证书签发超时 (120秒)"));
            }

            order = account
                .order(&client_config, &order_url)
                .await
                .map_err(|e| crate::Error::custom(50020, format!("ACME: 查询订单失败: {}", e)))?;

            match &order.status {
                rustls_acme::acme::OrderStatus::Valid { certificate } => {
                    break certificate.clone();
                }
                rustls_acme::acme::OrderStatus::Invalid => {
                    #[cfg(feature = "log")]
                    tracing::error!("ACME:   证书签发失败, 订单状态: {:?}", order.status);
                    return Err(crate::Error::custom(
                        50020,
                        "ACME: 订单最终化失败".to_string(),
                    ));
                }
                _ => {
                    #[cfg(feature = "log")]
                    tracing::debug!(
                        "ACME:   订单状态: {:?}, 等待签发... ({}/60)",
                        order.status,
                        attempts
                    );
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        };

        // ── 7. 获取证书 ──
        #[cfg(feature = "log")]
        tracing::info!("ACME: [7/7] 下载并保存证书...");

        let cert_pem = account
            .certificate(&client_config, &certificate_url)
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 获取证书失败: {}", e)))?;

        let key_pem = key_pair.serialize_pem();

        // ── 写入文件 ──
        tokio::fs::write(&cert_path, &cert_pem)
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 写入证书失败: {}", e)))?;
        tokio::fs::write(&key_path, &key_pem)
            .await
            .map_err(|e| crate::Error::custom(50020, format!("ACME: 写入私钥失败: {}", e)))?;

        // 清理挑战存储
        self.challenge_store.write().await.clear();

        #[cfg(feature = "log")]
        tracing::info!("ACME: ✅ 证书申请完成! {:?} / {:?}", cert_path, key_path);

        Ok((cert_path, key_path))
    }

    /// 调用证书获取成功回调
    ///
    /// 由框架内部在证书首次申请成功后调用。
    pub async fn invoke_cert_obtained(
        &self,
        state: &crate::AppState,
        cert_path: PathBuf,
        key_path: PathBuf,
    ) {
        if let Some(ref cb) = self.runtime.on_cert_obtained {
            let event = CertEvent {
                cert_path,
                key_path,
                domains: self.config.domains.clone(),
            };
            if let Err(e) = cb((state.clone(), event)).await {
                #[cfg(feature = "log")]
                tracing::error!("ACME: 证书获取回调执行失败: {}", e);
            }
        }
    }

    /// 调用证书续期成功回调
    ///
    /// 由框架内部在证书自动续期成功后调用。
    pub async fn invoke_cert_renewed(
        &self,
        state: &crate::AppState,
        cert_path: PathBuf,
        key_path: PathBuf,
    ) {
        if let Some(ref cb) = self.runtime.on_cert_renewed {
            let event = CertEvent {
                cert_path,
                key_path,
                domains: self.config.domains.clone(),
            };
            if let Err(e) = cb((state.clone(), event)).await {
                #[cfg(feature = "log")]
                tracing::error!("ACME: 证书续期回调执行失败: {}", e);
            }
        }
    }

    /// 调用证书失败回调
    ///
    /// 由框架内部在证书申请或续期失败后调用。
    pub async fn invoke_cert_failed(&self, state: &crate::AppState, error: &str) {
        if let Some(ref cb) = self.runtime.on_cert_failed {
            let event = CertErrorEvent {
                error: error.to_string(),
                domains: self.config.domains.clone(),
            };
            if let Err(e) = cb((state.clone(), event)).await {
                #[cfg(feature = "log")]
                tracing::error!("ACME: 证书失败回调执行失败: {}", e);
            }
        }
    }

    /// 启动证书续期后台任务
    ///
    /// 立即检查一次证书是否需要续期（到期或域名变更），之后每 24 小时检查一次。
    /// 续期成功后调用 `on_cert_renewed` 回调（如果已注册），
    /// 用户可在回调中调用 `state.tls.reload()` 触发 HTTPS 热重载。
    pub fn spawn_renewal_task(&self, state: crate::AppState) {
        let acme = self.clone();
        let renewal_interval = Duration::from_secs(24 * 60 * 60); // 24 小时
        let renewal_days = acme.config.renewal_days;

        tokio::spawn(async move {
            loop {
                if acme.has_cached_certs() {
                    // 检查是否因到期需要续期
                    let need_renew = match check_cert_expiry(&acme.cert_path(), renewal_days).await
                    {
                        Ok(true) => {
                            #[cfg(feature = "log")]
                            tracing::info!("ACME: 证书即将到期，需要续期");
                            true
                        }
                        Ok(false) => false,
                        Err(e) => {
                            #[cfg(feature = "log")]
                            tracing::warn!("ACME: 检查证书到期时间失败: {}", e);
                            false
                        }
                    };

                    // 检查域名是否有新增或变更（从证书 SAN 扩展中提取对比）
                    let domains_changed = match check_domains_changed(
                        &acme.cert_path(),
                        &acme.config.domains,
                    )
                    .await
                    {
                        Ok(true) => {
                            #[cfg(feature = "log")]
                            tracing::info!("ACME: 域名有新增或变更，需要重新申请证书");
                            true
                        }
                        Ok(false) => false,
                        Err(e) => {
                            #[cfg(feature = "log")]
                            tracing::warn!("ACME: 检查域名变更失败: {}", e);
                            false
                        }
                    };

                    if need_renew || domains_changed {
                        #[cfg(feature = "log")]
                        tracing::info!(
                            "ACME: 开始重新申请证书 (到期={}, 域名变更={})...",
                            need_renew,
                            domains_changed
                        );

                        let _ = tokio::fs::remove_file(acme.cert_path()).await;
                        let _ = tokio::fs::remove_file(acme.key_path()).await;

                        match acme.obtain_certificates().await {
                            Ok((cert_path, key_path)) => {
                                #[cfg(feature = "log")]
                                tracing::info!("ACME: 证书申请成功");
                                acme.invoke_cert_renewed(&state, cert_path, key_path).await;
                            }
                            Err(e) => {
                                #[cfg(feature = "log")]
                                tracing::error!("ACME: 证书申请失败: {}", e);
                                acme.invoke_cert_failed(&state, &e.to_string()).await;
                            }
                        }
                    } else {
                        #[cfg(feature = "log")]
                        tracing::debug!("ACME: 证书有效且域名未变，无需续期");
                    }
                }

                tokio::time::sleep(renewal_interval).await;
            }
        });
    }

    /// 初始化证书：检查缓存或在后台申请
    ///
    /// - 缓存命中且域名未变 → 启动续期后台任务，返回 `Some((cert_path, key_path))`
    /// - 缓存命中但域名有新增/变更 → 删除旧证书，后台重新申请，返回 `None`
    /// - 缓存未命中 → 在后台申请，申请成功后自动启动续期任务，返回 `None`
    pub fn init_certs(&self, state: crate::AppState) -> Option<(PathBuf, PathBuf)> {
        if self.has_cached_certs() {
            #[cfg(feature = "log")]
            tracing::debug!(
                "ACME: 发现缓存证书，后台检查域名是否变更 (当前: {:?})...",
                self.config.domains
            );

            // 后台检查域名是否有新增或变更
            let cert_path = self.cert_path();
            let current_domains = self.config.domains.clone();
            let acme = self.clone();

            tokio::spawn(async move {
                let domains_changed =
                    match check_domains_changed(&cert_path, &current_domains).await {
                        Ok(changed) => changed,
                        Err(e) => {
                            #[cfg(feature = "log")]
                            tracing::warn!("ACME: 检查域名变更失败: {}", e);
                            false
                        }
                    };

                if domains_changed {
                    #[cfg(feature = "log")]
                    tracing::debug!("ACME: 域名有新增或变更，删除旧证书并重新申请...");
                    let _ = tokio::fs::remove_file(acme.cert_path()).await;
                    let _ = tokio::fs::remove_file(acme.key_path()).await;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    match acme.obtain_certificates().await {
                        Ok((cert_path, key_path)) => {
                            #[cfg(feature = "log")]
                            tracing::info!("ACME: 证书申请成功");
                            acme.invoke_cert_obtained(&state, cert_path, key_path).await;
                            acme.spawn_renewal_task(state);
                        }
                        Err(e) => {
                            #[cfg(feature = "log")]
                            tracing::error!("ACME: 证书申请失败: {}", e);
                            acme.invoke_cert_failed(&state, &e.to_string()).await;
                        }
                    }
                } else {
                    #[cfg(feature = "log")]
                    tracing::debug!("ACME: 域名未变更，使用缓存证书");
                    acme.spawn_renewal_task(state);
                }
            });

            // 旧证书仍然返回，让服务先用旧证书启动（后台会在完成后通过回调通知热重载）
            Some((self.cert_path(), self.key_path()))
        } else {
            #[cfg(feature = "log")]
            tracing::debug!(
                "ACME: 证书未缓存，2 秒后开始为 {:?} 申请证书 ({}环境)...",
                self.config.domains,
                if self.config.staging {
                    "测试"
                } else {
                    "生产"
                }
            );

            let acme = self.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(2)).await;
                match acme.obtain_certificates().await {
                    Ok((cert_path, key_path)) => {
                        #[cfg(feature = "log")]
                        tracing::info!("ACME: 证书申请成功");
                        acme.invoke_cert_obtained(&state, cert_path, key_path).await;
                        // 首次申请成功后，启动续期任务
                        acme.spawn_renewal_task(state);
                    }
                    Err(e) => {
                        #[cfg(feature = "log")]
                        tracing::error!("ACME: 证书申请失败: {}", e);
                        acme.invoke_cert_failed(&state, &e.to_string()).await;
                    }
                }
            });
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  HTTP Handler
// ═══════════════════════════════════════════════════════════════

/// ACME HTTP-01 挑战响应 handler
///
/// 注册在 afast 的 HTTP 服务上，路径为 `.well-known/acme-challenge/*`。
/// 当 Let's Encrypt 请求 `http://<domain>/.well-known/acme-challenge/<token>` 时，
/// 从 `ChallengeStore` 中查找对应的 `key_authorization` 并返回。
#[afast::get(desc("ACME HTTP-01 Challenge"), no_trace)]
pub async fn challenge_handler(
    afast::FullPath(path): afast::FullPath,
    afast::State(state): afast::State<crate::AppState>,
) -> afast::HttpResult<afast::Text> {
    let acme = &state.acme;

    // 从路径中提取 token
    let token = path
        .strip_prefix("/.well-known/acme-challenge/")
        .or_else(|| path.strip_prefix(".well-known/acme-challenge/"))
        .unwrap_or("");

    if token.is_empty() {
        return Err(afast::Error::custom(400, "Missing challenge token"));
    }

    let store = acme.challenge_store.read().await;
    match store.get(token) {
        Some(key_auth) => {
            #[cfg(feature = "log")]
            tracing::debug!(
                "ACME: challenge 响应 token={}, key_auth={}",
                token,
                key_auth
            );
            Ok(afast::Text(key_auth.clone()))
        }
        None => Err(afast::Error::custom(404, "Challenge not found")),
    }
}

// ═══════════════════════════════════════════════════════════════
//  辅助函数
// ═══════════════════════════════════════════════════════════════

/// 检查证书是否即将到期
///
/// 读取 PEM 证书文件，解析 `not_after` 字段判断真实过期时间。
/// 返回 `Ok(true)` 表示证书需要续期。
async fn check_cert_expiry(cert_path: &PathBuf, renewal_days: u32) -> crate::Result<bool> {
    let pem_data = tokio::fs::read(cert_path)
        .await
        .map_err(|e| crate::Error::custom(50020, format!("ACME: 读取证书文件失败: {}", e)))?;

    let cert = x509_certificate::X509Certificate::from_pem(&pem_data)
        .map_err(|e| crate::Error::custom(50020, format!("ACME: 解析证书失败: {}", e)))?;

    let not_after = cert.validity_not_after();
    let now = chrono::Utc::now();

    if not_after <= now {
        #[cfg(feature = "log")]
        tracing::warn!("ACME: 证书已过期! 过期时间={}", not_after);
        return Ok(true);
    }

    let remaining = (not_after - now).num_days();
    let renewal_threshold = renewal_days as i64;

    #[cfg(feature = "log")]
    tracing::debug!(
        "ACME: 证书过期时间={}, 剩余={}天, 续期阈值={}天",
        not_after,
        remaining,
        renewal_days
    );

    Ok(remaining <= renewal_threshold)
}

/// 从证书的 SAN (Subject Alternative Name) 扩展中提取 DNS 域名列表
///
/// 解析 X.509 证书中 OID 2.5.29.17 的扩展值，
/// 提取所有 dNSName 条目。
fn extract_san_domains(cert: &x509_certificate::X509Certificate) -> Vec<String> {
    let mut domains = Vec::new();

    for ext in cert.iter_extensions() {
        // OID 2.5.29.17 = subjectAltName
        if ext.id.to_string() != "2.5.29.17" {
            continue;
        }

        // 获取扩展值的原始 DER 字节
        // 结构: OCTET STRING (extnValue) 包裹 SEQUENCE OF GeneralName
        let bytes = ext.value.to_bytes();
        if bytes.len() < 2 || bytes[0] != 0x30 {
            continue;
        }

        // 跳过外层 SEQUENCE 标签和长度，进入 GeneralNames 内容
        let (inner_start, inner_end) = match read_der_tlv(&bytes, 0) {
            Some((start, end)) => (start, end),
            None => continue,
        };

        // 解析 SEQUENCE 内的 GeneralName 条目
        let mut pos = inner_start;
        let slice = &bytes;
        while pos < inner_end && pos < slice.len() {
            if pos >= slice.len() {
                break;
            }
            let tag = slice[pos];

            // 读取 DER TLV
            let (content_start, content_end) = match read_der_tlv(slice, pos) {
                Some((s, e)) => (s, e),
                None => break,
            };

            // dNSName: context-specific [2], primitive → tag = 0x82
            if tag == 0x82 {
                let name = String::from_utf8_lossy(&slice[content_start..content_end]).to_string();
                domains.push(name);
            }

            pos = content_end;
        }
        break; // 只处理第一个 SAN 扩展
    }

    domains
}

/// 读取 DER TLV 结构，返回 (内容起始位置, 内容结束位置)
/// 返回 None 表示解析失败
fn read_der_tlv(data: &[u8], start: usize) -> Option<(usize, usize)> {
    if start >= data.len() {
        return None;
    }
    let mut pos = start + 1; // 跳过 tag
    if pos >= data.len() {
        return None;
    }
    let len_byte = data[pos];
    pos += 1;

    let length: usize = if len_byte & 0x80 == 0 {
        // 短格式：长度 < 128
        len_byte as usize
    } else {
        // 长格式：低 7 位表示后续字节数
        let num_bytes = (len_byte & 0x7f) as usize;
        let mut len_val = 0usize;
        for _ in 0..num_bytes {
            if pos >= data.len() {
                return None;
            }
            len_val = (len_val << 8) | (data[pos] as usize);
            pos += 1;
        }
        len_val
    };

    let content_start = pos;
    let content_end = pos + length;
    if content_end > data.len() {
        return None;
    }
    Some((content_start, content_end))
}

/// 检查证书中的域名与当前配置的域名是否有新增或变更
///
/// 从缓存的 PEM 证书中提取 SAN 域名，与当前配置的域名集合对比：
/// - 如果当前域名集合中存在证书中没有的域名 → 返回 `Ok(true)`（需要重新申请）
/// - 如果当前域名是证书域名的子集（只减少没新增） → 返回 `Ok(false)`
async fn check_domains_changed(
    cert_path: &PathBuf,
    current_domains: &[String],
) -> crate::Result<bool> {
    let pem_data = tokio::fs::read(cert_path)
        .await
        .map_err(|e| crate::Error::custom(50020, format!("ACME: 读取证书文件失败: {}", e)))?;

    let cert = x509_certificate::X509Certificate::from_pem(&pem_data)
        .map_err(|e| crate::Error::custom(50020, format!("ACME: 解析证书失败: {}", e)))?;

    let cert_domains = extract_san_domains(&cert);
    let cert_set: HashSet<&str> = cert_domains.iter().map(|s| s.as_str()).collect();
    let current_set: HashSet<&str> = current_domains.iter().map(|s| s.as_str()).collect();

    // 当前有而证书中没有的域名 → 需要重新申请
    let new_domains: Vec<_> = current_set.difference(&cert_set).collect();
    if !new_domains.is_empty() {
        #[cfg(feature = "log")]
        tracing::debug!(
            "ACME: 检测到新增/变更域名: {:?} (证书现有: {:?})",
            new_domains,
            cert_domains
        );
        return Ok(true);
    }

    #[cfg(feature = "log")]
    tracing::debug!("ACME: 域名未变更 (证书: {:?})", cert_domains);

    Ok(false)
}

// ═══════════════════════════════════════════════════════════════
//  AFaster 构建器扩展
// ═══════════════════════════════════════════════════════════════

/// AFaster ACME 配置扩展
pub trait AFasterAcmeExt {
    /// 链式配置 ACME
    fn with_acme(self, f: impl FnOnce(AcmeState) -> AcmeState) -> Self;
}

impl AFasterAcmeExt for crate::AFaster {
    fn with_acme(mut self, f: impl FnOnce(AcmeState) -> AcmeState) -> Self {
        self.state.acme = f(self.state.acme);
        self
    }
}
