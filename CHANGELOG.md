# Changelog

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
