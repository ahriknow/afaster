# 邮件发送

Feature: `email`

支持配置多个 SMTP 邮箱账户，每个账户通过唯一 ID 标识。发送时可按 ID 指定使用哪个账户。

## 配置

```toml
[email]
default = "main"          # 默认邮箱 ID

[[email.accounts]]
id     = "main"           # 唯一标识（不可重复）
host   = "smtp.qq.com"    # SMTP 服务器地址
port   = 465              # 端口（465=SSL, 587=STARTTLS）
user   = ""               # 登录用户名
pass   = ""               # 密码/授权码
from   = ""               # 发件人地址
name   = ""               # 发件人显示名称
# secure = true            # SSL/TLS, 默认 true
# starttls = false         # STARTTLS 模式（587 端口），默认 false

# [[email.accounts]]
# id     = "notify"
# host   = "smtp.163.com"
# port   = 465
# user   = ""
# pass   = ""
# from   = ""
# name   = "通知服务"
```

## 校验规则

- `id` 不可重复，启动时检查，重复则报错 `41002`
- `default` 必须是 `accounts` 中已存在的 ID，否则报错 `41003`

## API

### Email

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `from_config` | `config: &EmailConfig` | `Result<Email>` | 从配置创建，校验 ID |
| `send` | `to, subject, body, cc` | `Result<()>` | 默认账户发送 HTML 邮件 |
| `send_with` | `id, to, subject, body, cc` | `Result<()>` | 指定账户发送 HTML 邮件 |
| `send_html` | `to, subject, body, cc` | `Result<()>` | 默认账户发送 HTML 邮件 |
| `send_html_with` | `id, to, subject, body, cc` | `Result<()>` | 指定账户发送 HTML 邮件 |
| `send_text` | `to, subject, body, cc` | `Result<()>` | 默认账户发送纯文本邮件 |
| `send_text_with` | `id, to, subject, body, cc` | `Result<()>` | 指定账户发送纯文本邮件 |

> `cc` 参数类型为 `&[&str]`，传 `&[]` 表示不抄送。`send` / `send_with` 与 `send_html` / `send_html_with` 等价。

## 错误码

| 码 | English | 中文 |
|----|---------|------|
| 41001 | Email account not found | 邮箱账户未找到 |
| 41002 | Duplicate email account ID | 邮箱账户 ID 重复 |
| 41003 | Default email account ID not found | 默认邮箱 ID 不存在 |
| 51001 | SMTP connection failed | SMTP 连接失败 |
| 51002 | Email send failed | 邮件发送失败 |

## 使用示例

```rust
// 使用默认账户发送 HTML 邮件
state.email.send("to@example.com", "标题", "<h1>正文</h1>", &[]).await?;

// 指定账户发送 HTML 邮件
state.email.send_with("notify", "to@example.com", "标题", "<h1>正文</h1>", &[]).await?;

// 发送纯文本邮件
state.email.send_text("to@example.com", "标题", "纯文本正文", &[]).await?;

// 带抄送发送
state.email.send("to@example.com", "标题", "<p>正文</p>", &["cc1@example.com", "cc2@example.com"]).await?;

// 指定账户 + 抄送 + 纯文本
state.email.send_text_with("notify", "to@example.com", "标题", "正文", &["cc@example.com"]).await?;
```

## 模块结构

```
src/state/email/
├── mod.rs   # Email 结构体、配置、发送逻辑
└── err.rs   # 邮件错误码函数
```
