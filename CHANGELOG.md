# Changelog

## [0.0.2]

### Added

- OSS: Added `get_signed_image_url`, `get_signed_style_url`, `get_signed_video_snapshot`, `get_signed_video_cover`
- COS: Added `get_signed_image_url`, `get_signed_style_url`, `get_signed_video_snapshot`, `get_signed_video_cover`

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
