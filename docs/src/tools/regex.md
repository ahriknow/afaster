# 正则工具

Feature: `regex-util`

## API

### 静态验证方法

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `is_email` | `s: &str` | `bool` | 验证邮箱地址 |
| `is_phone` | `s: &str` | `bool` | 验证中国大陆手机号（1 开头 11 位） |
| `is_phone_cn` | `s: &str` | `bool` | 验证手机号（带可选 +86 前缀） |
| `is_url` | `s: &str` | `bool` | 验证 URL（http/https） |
| `is_ipv4` | `s: &str` | `bool` | 验证 IPv4 地址 |
| `is_ipv6` | `s: &str` | `bool` | 验证 IPv6 地址 |
| `is_id_card` | `s: &str` | `bool` | 验证中国大陆身份证号（18 位，仅格式） |
| `validate_id_card` | `s: &str` | `bool` | 严格验证身份证号（格式 + 校验码 + 生日合法性） |
| `is_username` | `s: &str` | `bool` | 验证用户名（字母/数字/下划线，3-32 位） |
| `is_strong_password` | `s: &str` | `bool` | 验证强密码（≥8 位，含大小写+数字） |
| `is_chinese` | `s: &str` | `bool` | 验证纯中文字符 |
| `is_hex_color` | `s: &str` | `bool` | 验证十六进制颜色值 |

### 通用匹配方法

| 方法 | 参数 | 返回 | 说明 |
|------|------|------|------|
| `is_match` | `pattern, s` | `Result<bool, String>` | 检查字符串是否匹配正则 |
| `find` | `pattern, s` | `Result<Option<String>, String>` | 查找第一个匹配 |
| `find_all` | `pattern, s` | `Result<Vec<String>, String>` | 查找所有匹配 |
| `captures` | `pattern, s` | `Result<Option<Vec<String>>, String>` | 提取捕获组 |
| `captures_all` | `pattern, s` | `Result<Vec<Vec<String>>, String>` | 提取所有匹配的捕获组 |
| `replace` | `pattern, s, rep` | `Result<String, String>` | 替换第一个匹配 |
| `replace_all` | `pattern, s, rep` | `Result<String, String>` | 替换所有匹配 |
| `split` | `pattern, s` | `Result<Vec<String>, String>` | 按正则分割字符串 |

## 使用示例

```rust
// 常用验证
let valid = RegexUtil::is_email("test@example.com");       // true
let valid = RegexUtil::is_phone("13800138000");            // true
let valid = RegexUtil::is_url("https://example.com");      // true
let valid = RegexUtil::is_id_card("11010119900101001X");   // true
let valid = RegexUtil::validate_id_card("11010119900101001X"); // true（含校验码验证）

// 通用匹配
let matched = RegexUtil::is_match(r"^\d{4}-\d{2}-\d{2}$", "2025-01-01")?;
let found = RegexUtil::find(r"\d+", "abc123def456")?;       // Some("123")
let all = RegexUtil::find_all(r"\d+", "abc123def456")?;     // ["123", "456"]

// 捕获组
let caps = RegexUtil::captures(r"(\d{4})-(\d{2})-(\d{2})", "2025-01-15")?;
// Some(["2025", "01", "15"])

// 替换
let result = RegexUtil::replace_all(r"\d+", "abc123def456", "*")?;
// "abc*def*"

// 分割
let parts = RegexUtil::split(r"[,;]", "a,b;c,d")?;
// ["a", "b", "c", "d"]
```
