# 环境变量配置

`src/state` 中的配置模块支持从环境变量读取配置。环境变量和 `config.toml` 同时设置时，环境变量优先；未设置的环境变量继续使用 TOML 值。TOML 配置文件、配置段或可选字段缺失时使用表中的默认值；必填字段缺失时，初始化会返回包含配置字段和环境变量名的错误。

字符串直接设置，数字和布尔值分别使用 Rust 标准数字格式及 `true` / `false`。列表环境变量使用逗号分隔。表中“必填”表示相应 feature 启用并调用该模块配置解析时必须提供；未启用的 feature 不读取其配置。

## Backend、数据库和 ORM

| TOML 字段 | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[backend].host` | `BACKEND_HOST` | 否 | `0.0.0.0` |
| `[backend].port` | `BACKEND_PORT` | 否 | `6080` |
| `[postgres].host` / `port` / `user` / `pass` / `name` | `POSTGRES_HOST` / `POSTGRES_PORT` / `POSTGRES_USER` / `POSTGRES_PASS` / `POSTGRES_NAME` | 否 | host `127.0.0.1`，port `5432`，user/name `postgres`，pass 为空 |
| `[mysql].host` / `port` / `user` / `pass` / `name` | `MYSQL_HOST` / `MYSQL_PORT` / `MYSQL_USER` / `MYSQL_PASS` / `MYSQL_NAME` | 否 | host `127.0.0.1`，port `3306`，user `root`，name `mysql`，pass 为空 |
| `[sqlite].path` | `SQLITE_PATH` | 否 | 为空时使用内存数据库 |
| `[orm-sqlite].path` / `token` / `url` | `ORM_SQLITE_PATH` / `ORM_SQLITE_TOKEN` / `ORM_SQLITE_URL` | 否 | path 为空时使用内存数据库；token/url 可选 |
| `[orm-postgres].host` / `port` / `user` / `pass` / `name` | `ORM_POSTGRES_HOST` / `ORM_POSTGRES_PORT` / `ORM_POSTGRES_USER` / `ORM_POSTGRES_PASS` / `ORM_POSTGRES_NAME` | host、user、name 是 | port `5432`，pass 为空 |
| `[orm-postgres].token` / `url` | `ORM_POSTGRES_TOKEN` / `ORM_POSTGRES_URL` | 否 | ORM 管理端凭证/路径，可选 |
| `[orm-mysql].host` / `port` / `user` / `pass` / `name` | `ORM_MYSQL_HOST` / `ORM_MYSQL_PORT` / `ORM_MYSQL_USER` / `ORM_MYSQL_PASS` / `ORM_MYSQL_NAME` | host、user、name 是 | port `3306`，pass 为空 |
| `[orm-mysql].token` / `url` | `ORM_MYSQL_TOKEN` / `ORM_MYSQL_URL` | 否 | ORM 管理端凭证/路径，可选 |
| ORM 管理端 token / URL | `ORM_ADMIN_TOKEN` / `ORM_ADMIN_URL` | 否 | 可覆盖对应 ORM section 的 token/url |

## 缓存、认证和文件服务

| TOML 字段 | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[redis].host` / `port` / `db` / `password` / `prefix` | `REDIS_HOST` / `REDIS_PORT` / `REDIS_DB` / `REDIS_PASSWORD` / `REDIS_PREFIX` | 否 | `127.0.0.1`、`6379`、`0`、空密码、空前缀 |
| `[token].secret` / `expire` | `TOKEN_SECRET` / `TOKEN_EXPIRE` | `TOKEN_SECRET` 是 | `expire` 默认为 `24` 小时 |
| `[file].root` / `max_size` / `allowed` / `url_prefix` | `FILE_ROOT` / `FILE_MAX_SIZE` / `FILE_ALLOWED` / `FILE_URL_PREFIX` | 否 | `./uploads`、10 MiB、空（不限制扩展名）、`/files`；`FILE_ALLOWED` 以逗号分隔 |
| `[tls].port` / `cert_path` / `key_path` | `TLS_PORT` / `TLS_CERT_PATH` / `TLS_KEY_PATH` | cert_path、key_path 是 | port 默认为 `443` |
| `[snow].epoch` / `worker_id` / `datacenter_id` | `SNOW_EPOCH` / `SNOW_WORKER_ID` / `SNOW_DATACENTER_ID` | 否 | `0`（当前时间）、`1`、`1` |
| `[bloom].expected_items` / `false_positive_rate` / `auto_check_key` | `BLOOM_EXPECTED_ITEMS` / `BLOOM_FALSE_POSITIVE_RATE` / `BLOOM_AUTO_CHECK_KEY` | 否 | `100000`、`0.01`、未设置 |

## ACME 证书

| TOML 字段 | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[acme].domains` | `ACME_DOMAINS` | 否 | 空列表时不配置域名；环境变量使用逗号分隔 |
| `[acme].contact` | `ACME_CONTACT` | 否 | 无默认联系人 |
| `[acme].cache_dir` | `ACME_CACHE_DIR` | 否 | `./acme_cache` |
| `[acme].staging` | `ACME_STAGING` | 否 | `false` |
| `[acme].renewal_days` | `ACME_RENEWAL_DAYS` | 否 | `10` |
| `[acme].allow_non_80` | `ACME_ALLOW_NON_80` | 否 | `false` |

## 限流、静态文件和邮件

| TOML 字段 | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[rate_limit].enabled` / `default_policy` / `rejected_code` / `rejected_message` | `RATE_LIMIT_ENABLED` / `RATE_LIMIT_DEFAULT_POLICY` / `RATE_LIMIT_REJECTED_CODE` / `RATE_LIMIT_REJECTED_MESSAGE` | 否 | `true`、无默认策略、`42001`、`Too many requests` |
| `[[rate_limit.policies]]` 的 `id` / `max_requests` / `window_seconds` / `key` / `algorithm` / `header_name` | `RATE_LIMIT_POLICIES_{INDEX}_ID` / `_MAX_REQUESTS` / `_WINDOW_SECONDS` / `_KEY` / `_ALGORITHM` / `_HEADER_NAME` | id、max_requests、window_seconds 是 | key 默认为 `ip`，algorithm 默认为 `token_bucket`；`INDEX` 从 `0` 开始，key 可为 `ip`、`header`、`connection`、`global` |
| `[serve]` 的 `prefix` / `spa` | `SERVE_PREFIX` / `SERVE_SPA` | 否 | 单个服务使用此名称；prefix 默认为 `/`，spa 默认为 `false` |
| `[[serve]]` 的 `prefix` / `spa` | `SERVE_{INDEX}_PREFIX` / `SERVE_{INDEX}_SPA` | 否 | 多个服务时 `INDEX` 从 `0` 开始 |
| `[email].default` | `EMAIL_DEFAULT` | 配置账户时是 | 需指向下方某个账户的 ID |
| `[[email.accounts]]` 的 `id` / `host` / `port` / `user` / `pass` / `from` / `name` / `secure` / `starttls` | `EMAIL_ACCOUNTS_{INDEX}_ID` / `_HOST` / `_PORT` / `_USER` / `_PASS` / `_FROM` / `_NAME` / `_SECURE` / `_STARTTLS` | id、host、user、pass、from 是 | `INDEX` 从 `0` 开始；port `465`、name 为空、secure `true`、starttls `false` |

## 云服务、OAuth、短信和推送

| TOML section / fields | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[amap].key` | `AMAP_KEY` | 是 | 高德地图 Web API key |
| `[tmap].key` | `TMAP_KEY` | 是 | 腾讯地图 Web API key |
| `[cos]` secret_id / secret_key / bucket / region | `COS_SECRET_ID` / `COS_SECRET_KEY` / `COS_BUCKET` / `COS_REGION` | 是 | COS 访问凭证和存储桶 |
| `[cos]` prefix / domain / url_expire | `COS_PREFIX` / `COS_DOMAIN` / `COS_URL_EXPIRE` | 否 | prefix/domain 为空，url_expire `3600` 秒 |
| `[oss]` access_key_id / access_key_secret / bucket / region | `OSS_ACCESS_KEY_ID` / `OSS_ACCESS_KEY_SECRET` / `OSS_BUCKET` / `OSS_REGION` | 是 | OSS 访问凭证和存储桶 |
| `[oss]` endpoint / url_expire / sts_expire / role_arn / prefix / domain | `OSS_ENDPOINT` / `OSS_URL_EXPIRE` / `OSS_STS_EXPIRE` / `OSS_ROLE_ARN` / `OSS_PREFIX` / `OSS_DOMAIN` | 否 | endpoint 为空（按 region 推导），url/sts expiry 均为 `3600` 秒 |
| `[sms_ali]` access_key_id / access_key_secret | `SMS_ALI_ACCESS_KEY_ID` / `SMS_ALI_ACCESS_KEY_SECRET` | 是 | 阿里云短信凭证 |
| `[sms_ali]` sign_name / template_code / callback_path | `SMS_ALI_SIGN_NAME` / `SMS_ALI_TEMPLATE_CODE` / `SMS_ALI_CALLBACK_PATH` | 否 | sign/template 可在发送时指定；callback path 默认为 `sms/ali/report` |
| `[sms_tencent]` secret_id / secret_key / sdk_app_id | `SMS_TENCENT_SECRET_ID` / `SMS_TENCENT_SECRET_KEY` / `SMS_TENCENT_SDK_APP_ID` | 是 | 腾讯云短信凭证 |
| `[sms_tencent]` sign_name / template_id / callback_path | `SMS_TENCENT_SIGN_NAME` / `SMS_TENCENT_TEMPLATE_ID` / `SMS_TENCENT_CALLBACK_PATH` | 否 | sign/template 可在发送时指定；callback path 默认为 `sms/tencent/report` |
| `[github_oauth2]` client_id / client_secret / redirect_base | `GITHUB_OAUTH2_CLIENT_ID` / `GITHUB_OAUTH2_CLIENT_SECRET` / `GITHUB_OAUTH2_REDIRECT_BASE` | 是 | GitHub OAuth 应用凭证及回调基址 |
| `[github_oauth2]` callback_path / scope / authorize_url / token_url / user_url / verify_state / state_expire | `GITHUB_OAUTH2_CALLBACK_PATH` / `GITHUB_OAUTH2_SCOPE` / `GITHUB_OAUTH2_AUTHORIZE_URL` / `GITHUB_OAUTH2_TOKEN_URL` / `GITHUB_OAUTH2_USER_URL` / `GITHUB_OAUTH2_VERIFY_STATE` / `GITHUB_OAUTH2_STATE_EXPIRE` | 否 | 使用内置 GitHub URL、默认 scope、state 验证关闭、有效期 `300` 秒 |
| `[push.getui]` app_id / app_key / master_secret | `PUSH_GETUI_APP_ID` / `PUSH_GETUI_APP_KEY` / `PUSH_GETUI_MASTER_SECRET` | 是 | 个推凭证 |
| `[push.getui].base_url` | `PUSH_GETUI_BASE_URL` | 否 | 使用个推默认 API URL |
| `[push.jpush]` app_key / master_secret | `PUSH_JPUSH_APP_KEY` / `PUSH_JPUSH_MASTER_SECRET` | 是 | 极光推送凭证 |
| `[push.jpush]` base_url / apns_production | `PUSH_JPUSH_BASE_URL` / `PUSH_JPUSH_APNS_PRODUCTION` | 否 | 使用极光默认 API URL；apns_production 默认 `true` |
| `[push.xiaomi]` package_name / app_secret | `PUSH_XIAOMI_PACKAGE_NAME` / `PUSH_XIAOMI_APP_SECRET` | 是 | 小米推送凭证 |
| `[push.xiaomi].base_url` | `PUSH_XIAOMI_BASE_URL` | 否 | 使用小米默认 API URL |

## 微信和支付

| TOML section / fields | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[wxlogin]` mini_id / mini_secret | `WXLOGIN_MINI_ID` / `WXLOGIN_MINI_SECRET` | 启用 `wx-login-mini` 时是 | 小程序登录凭证 |
| `[wxlogin]` app_id / app_secret | `WXLOGIN_APP_ID` / `WXLOGIN_APP_SECRET` | 启用 `wx-login-app` 时是 | 微信 App 登录凭证 |
| `[wxlogin]` web_id / web_secret / redirect_base / callback_path | `WXLOGIN_WEB_ID` / `WXLOGIN_WEB_SECRET` / `WXLOGIN_REDIRECT_BASE` / `WXLOGIN_CALLBACK_PATH` | web_id、web_secret、redirect_base 在启用 `wx-login-web` 时是 | callback path 默认为 `auth/wechat/callback` |
| `[wx_official]` app_id / app_secret / base_url / redirect_base / callback_path | `WX_OFFICIAL_APP_ID` / `WX_OFFICIAL_APP_SECRET` / `WX_OFFICIAL_BASE_URL` / `WX_OFFICIAL_REDIRECT_BASE` / `WX_OFFICIAL_CALLBACK_PATH` | app_id、app_secret 是 | base URL 和 callback path 使用内置默认值；redirect_base 可选 |
| `[wx_pay]` mch_id / app_id / api_v3_key / private_key / serial_no | `WX_PAY_MCH_ID` / `WX_PAY_APP_ID` / `WX_PAY_API_V3_KEY` / `WX_PAY_PRIVATE_KEY` / `WX_PAY_SERIAL_NO` | 是 | 微信支付商户配置 |
| `[wx_pay]` notify_url / refund_notify_url / platform_cert / callback_path | `WX_PAY_NOTIFY_URL` / `WX_PAY_REFUND_NOTIFY_URL` / `WX_PAY_PLATFORM_CERT` / `WX_PAY_CALLBACK_PATH` | 否 | 支付/退款通知 URL 和平台证书可选；callback path 有默认值 |
| `[wx_sec_check]` app_id / app_secret | `WX_SEC_CHECK_APP_ID` / `WX_SEC_CHECK_APP_SECRET` | 是 | 微信内容安全凭证 |
| `[wx_sec_check]` base_url / callback_path / verify_token / msg_secret | `WX_SEC_CHECK_BASE_URL` / `WX_SEC_CHECK_CALLBACK_PATH` / `WX_SEC_CHECK_VERIFY_TOKEN` / `WX_SEC_CHECK_MSG_SECRET` | 否 | API URL 和 callback path 使用默认值；验签字段按需配置 |
| `[wx_virtual_pay]` offer_id / app_key / mini_id / mini_secret | `WX_VIRTUAL_PAY_OFFER_ID` / `WX_VIRTUAL_PAY_APP_KEY` / `WX_VIRTUAL_PAY_MINI_ID` / `WX_VIRTUAL_PAY_MINI_SECRET` | 是 | 虚拟支付及用于 session 的小程序凭证 |
| `[wx_virtual_pay]` env / callback_path / verify_token / msg_secret | `WX_VIRTUAL_PAY_ENV` / `WX_VIRTUAL_PAY_CALLBACK_PATH` / `WX_VIRTUAL_PAY_VERIFY_TOKEN` / `WX_VIRTUAL_PAY_MSG_SECRET` | 否 | env 默认为 `0`；callback path 有默认值；验签字段按需配置 |
| `[ali_pay]` app_id / private_key | `ALI_PAY_APP_ID` / `ALI_PAY_PRIVATE_KEY` | 是 | 支付宝应用 ID 和商户私钥 |
| `[ali_pay]` alipay_public_key / notify_url / return_url / gateway / charset / sign_type / callback_path | `ALI_PAY_ALIPAY_PUBLIC_KEY` / `ALI_PAY_NOTIFY_URL` / `ALI_PAY_RETURN_URL` / `ALI_PAY_GATEWAY` / `ALI_PAY_CHARSET` / `ALI_PAY_SIGN_TYPE` / `ALI_PAY_CALLBACK_PATH` | 否 | 公钥及 URL 可选；gateway、charset、sign type 和 callback path 有默认值 |

## 链路追踪

| TOML section / fields | 环境变量 | 必填 | 默认值/说明 |
|---|---|:---:|---|
| `[tracing]` service_name / db_path / url / event / retention_days | `TRACING_SERVICE_NAME` / `TRACING_DB_PATH` / `TRACING_URL` / `TRACING_EVENT` / `TRACING_RETENTION_DAYS` | 否 | service name `afaster`；SQLite 路径 `tracing.db`；UI 路径及保留天数采用默认值 |
| `[tracing.http]` url / path / token / timeout | `TRACING_HTTP_URL` / `TRACING_HTTP_PATH` / `TRACING_HTTP_TOKEN` / `TRACING_HTTP_TIMEOUT` | 启用 HTTP 存储配置时 url 是 | path `/tracing/report`，timeout `5` 秒，token 可选 |
| `[tracing.tcp]` addr / connect_timeout / reconnect_interval | `TRACING_TCP_ADDR` / `TRACING_TCP_CONNECT_TIMEOUT` / `TRACING_TCP_RECONNECT_INTERVAL` | 启用 TCP 存储配置时 addr 是 | connect timeout `5` 秒，reconnect interval `3` 秒 |

## 示例

```powershell
$env:BACKEND_PORT = "8080"
$env:REDIS_HOST = "127.0.0.1"
$env:TOKEN_SECRET = "replace-with-a-random-secret"
cargo run
```

不要把生产密钥提交到仓库。设置了某个环境变量时，即使它的值为空，也会优先于 TOML；配置凭证时请提供有效值。
