# Changelog

## [0.0.4]

### Added

- **ACME**: 新增 `acme` feature — 自动 HTTPS 证书管理（Let's Encrypt），支持 HTTP-01 验证
- **ACME**: `config.toml` 新增 `[acme]` 配置段，支持域名列表、联系邮箱、缓存目录、测试环境、续期天数
- **ACME**: 支持 `with_on_cert_obtained` / `with_on_cert_renewed` / `with_on_cert_failed` 回调
- **ACME**: 证书续期后台任务 — 立即检查 + 每 24 小时检查一次，到期前 `renewal_days` 天自动续期
- **ACME**: 首次申请成功后自动启动续期任务
- **TLS**: 从 `backend` 配置独立为 `[tls]` 模块，支持证书热重载（`state.tls.reload()`）
- **TLS**: 新增 `afast-tls` feature — HTTPS / WSS 支持，基于 rustls 实现

### Changed

- **架构**: `impl AFaster` 拆分 — 各模块的构建方法移至对应子模块，通过 extension trait 实现
- **架构**: `AFaster` 结构体字段改为 `pub(crate)`，子模块可直接访问
- **架构**: `AppState::new` 重构 — 每个模块新增 `from_table(&toml::Table)` 方法，自行从 TOML 提取配置并初始化
- **架构**: 移除 `ConfigFile` 中间结构体，`AppState::new` 直接通过 `from_table` 组装
- **架构**: 移除 `AppState` 上的 `with_*` / `set_*` 中间方法，AFaster extension trait 直接操作 `self.state` 字段
- **架构**: Scheduler `init_state` 从 `AppState::new` 移至 `AFaster::run()`，打破循环依赖
- **错误码**: 重新分配所有模块错误码（`XYYZZ` 格式），修复冲突，新增 ACME (31)、TLS (32)、链路追踪 (30)、定时任务 (35) 等模块编号
- **错误码**: 腾讯云短信从 11 拆出独立编号 17
- **错误码**: Excel 从 16 拆出独立编号 18
- **错误码**: 数据库从 09 拆出独立编号 25

## [0.0.3]

### Added

- TLS: 新增 `afast-tls` feature — HTTPS / WSS 支持，基于 rustls 实现
- TLS: `config.toml` 新增 `[backend.tls]` 配置段，支持自定义端口、证书链和私钥路径
- Serve: 新增 `serve` feature — 静态文件服务模块，支持运行时目录模式和编译期嵌入模式
- Serve: 新增 `serve-embed` feature — 使用 `include_dir` 在编译期将整个目录嵌入二进制文件
- Serve: 支持 SPA 模式（未找到文件时自动回退到 `index.html`）
- Serve: 支持自定义 URL 前缀（如 `/app`）
- Serve: 自动 MIME 类型推断（基于文件扩展名）
- Serve: 路径遍历安全防护
- Bloom Filter: `auto_check_key` 新增 `"client_ip"` 和 `"forwarded_for"` 选项，支持按真实 IP 或代理 IP 进行过滤
- Bloom Filter: `"ip"` 选项语义更新——优先真实 IP (`X-Forwarded-For` / `X-Real-IP`)，无代理时回退到代理 IP (TCP 直连地址)
- Bloom Filter: 新增 `docs/src/data/bloom.md` 文档页，包含配置说明、使用示例和 API 参考

## [0.0.2]

### Added

- OSS: Added `get_signed_image_url`, `get_signed_style_url`, `get_signed_video_snapshot`, `get_signed_video_cover`
- COS: Added `get_signed_image_url`, `get_signed_style_url`, `get_signed_video_snapshot`, `get_signed_video_cover`
- Trace: Added `trace-receive-http` feature — enables `afast-http` transport so external afaster projects can report spans via existing `report`/`report_batch` binary handlers
- Trace: Added `trace-receive-tcp` feature — enables `afast-tcp` transport so external afaster projects can report spans via existing `report`/`report_batch` binary handlers
- Trace: `trace-http` sender now uses afastdata binary protocol instead of JSON, config field renamed from `api_prefix` to `path`

### Fixed

- Trace UI: Fixed `formatTime`/`formatDateTime` treating microsecond timestamps as milliseconds, causing incorrect date display
- Trace UI: `formatDuration` now appends raw microsecond value (e.g. `1.5ms (1,500μs)`) for precision
- Trace UI: Improved font color readability for treemap labels, span tree durations, and idle gap indicators
- Trace UI: Treemap now uses dynamic scale (`calcScale`) based on root span total duration instead of fixed 100px/ms
- Trace UI: `<1ms` span minimum width is now dynamic, preventing visual inconsistency where short spans appear wider than 1ms spans
- Trace UI: Root treemap cell now has tooltip data attributes, fixing missing tooltip on hover
- Trace: Fixed data cleanup retention cutoff using millisecond multiplier with microsecond timestamps (`86400*1000` → `86400*1_000_000`)
- Database: 支持同时启用多个数据库（`db-postgres` / `db-sqlite` / `db-mysql`），不再互斥
- Database: 单数据库时可通过 `state.db.pool()` 获取连接池，多数据库时用 `state.db.pg()` / `state.db.sqlite()` / `state.db.mysql()`
- WxLogin: 修复 `mini_login` 和 `app_login` 每次请求创建新 `reqwest::Client` 导致连接泄漏的问题，改为复用 `self.client`
- Snowflake: 修复 `next_id()` 高并发竞态条件，改用 `tokio::sync::Mutex` 保护临界区
- Snowflake: `wait_next_millis` 忙等待中加入 `yield_now()` 避免 CPU 空转
- AppState: `new()` 方法改用 `Result` 返回错误，不再使用 `expect()` panic
- AppState: 配置文件读取改用 `tokio::fs::read_to_string` 避免阻塞 async 线程
- Error: `Json` 错误变体补全 `cos` 和 `sms-tencent` feature 门控
- OSS: `generate_nonce()` 加入原子计数器，避免高并发下 nonce 碰撞
- Health: 移除 `println!`，改用 `tracing::info!`

### Changed

- Snowflake: `next_id()` / `next_id_str()` / `next_id_prefix()` 改为 async 方法

## [0.0.1]

### Added

- Initial release
- Core framework: `AFaster` application builder with `config.toml` configuration
- Authentication: JWT, GitHub OAuth2, RBAC, Argon2 password hashing, rate limiting
- WeChat ecosystem: login (mini/APP/web), payment (H5/Native/APP/mini/JS), virtual pay, official account, content security
- Alibaba Cloud: OSS (image processing, video snapshot), SMS, Alipay web payment
- Tencent Cloud: COS (image processing, video snapshot), SMS, TMap
- Push notifications: GeTui, JPush, Xiaomi
- Data & cache: PostgreSQL, SQLite, MySQL, Redis, Valkey, MemKV, scheduler
- File processing: file service, image generation, Excel import/export, PDF generation (user-provided fonts)
- Tools: Snowflake ID, clock, regex utilities, tracing (SQLite/HTTP/TCP backends), logging
- Socket: WebSocket, binary socket, SSE support
- Maps: Amap, TMap
- Email: SMTP support
- Web UI: Tracing dashboard with real-time SSE updates
- Documentation: mdBook-based docs with Catppuccin theme
