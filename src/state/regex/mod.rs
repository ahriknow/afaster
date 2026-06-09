use regex::Regex;

// ═══════════════════════════════════════════════════════════════
//  正则工具
// ═══════════════════════════════════════════════════════════════

/// 正则工具模块
///
/// 提供常用正则验证和通用正则匹配能力。
#[derive(Clone)]
pub struct RegexUtil;

impl RegexUtil {
    pub fn new() -> Self {
        RegexUtil
    }

    // ── 常用验证 ──────────────────────────────────────────────

    /// 验证邮箱地址
    pub fn is_email(s: &str) -> bool {
        Regex::new(r"^[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}$")
            .unwrap()
            .is_match(s)
    }

    /// 验证中国大陆手机号（1 开头 11 位数字）
    pub fn is_phone(s: &str) -> bool {
        Regex::new(r"^1[3-9]\d{9}$").unwrap().is_match(s)
    }

    /// 验证中国大陆手机号（带 +86 前缀可选）
    pub fn is_phone_cn(s: &str) -> bool {
        Regex::new(r"^(\+86)?1[3-9]\d{9}$").unwrap().is_match(s)
    }

    /// 验证 URL（http/https）
    pub fn is_url(s: &str) -> bool {
        Regex::new(r"^https?://[^\s/$.?#].[^\s]*$")
            .unwrap()
            .is_match(s)
    }

    /// 验证 IPv4 地址
    pub fn is_ipv4(s: &str) -> bool {
        Regex::new(r"^(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)$")
            .unwrap()
            .is_match(s)
    }

    /// 验证 IPv6 地址（简化版）
    pub fn is_ipv6(s: &str) -> bool {
        Regex::new(r"^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$")
            .unwrap()
            .is_match(s)
    }

    /// 验证中国大陆身份证号（18 位，仅格式）
    pub fn is_id_card(s: &str) -> bool {
        Regex::new(r"^\d{17}[\dXx]$").unwrap().is_match(s)
    }

    /// 严格验证中国大陆身份证号（格式 + 校验码 + 生日合法性）
    ///
    /// 校验码算法（GB 11643-1999）：
    /// 前 17 位加权求和 mod 11，映射到校验码表 "10X98765432"
    pub fn validate_id_card(s: &str) -> bool {
        let s = s.to_ascii_uppercase();
        if !Regex::new(r"^\d{17}[\dX]$").unwrap().is_match(&s) {
            return false;
        }
        let bytes = s.as_bytes();
        // 前 6 位：地区码，简单检查非全 0
        if &bytes[..6] == b"000000" {
            return false;
        }
        // 第 7-14 位：出生日期 YYYYMMDD
        let year: u32 = std::str::from_utf8(&bytes[6..10])
            .unwrap()
            .parse()
            .unwrap_or(0);
        let month: u32 = std::str::from_utf8(&bytes[10..12])
            .unwrap()
            .parse()
            .unwrap_or(0);
        let day: u32 = std::str::from_utf8(&bytes[12..14])
            .unwrap()
            .parse()
            .unwrap_or(0);
        if year < 1900 || year > 2100 || month < 1 || month > 12 || day < 1 || day > 31 {
            return false;
        }
        // 每月天数校验
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                    29
                } else {
                    28
                }
            }
            _ => return false,
        };
        if day > max_day {
            return false;
        }
        // 校验码：加权求和 mod 11
        let weights: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
        let check_chars = b"10X98765432";
        let sum: u32 = (0..17).fold(0, |acc, i| acc + (bytes[i] - b'0') as u32 * weights[i]);
        let expected = check_chars[(sum % 11) as usize];
        bytes[17] == expected
    }

    /// 验证用户名（字母、数字、下划线，3-32 位）
    pub fn is_username(s: &str) -> bool {
        Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]{2,31}$")
            .unwrap()
            .is_match(s)
    }

    /// 验证强密码（至少 8 位，包含大小写字母和数字）
    pub fn is_strong_password(s: &str) -> bool {
        if s.len() < 8 {
            return false;
        }
        let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
        let has_digit = s.chars().any(|c| c.is_ascii_digit());
        has_upper && has_lower && has_digit
    }

    /// 验证中文字符（纯中文）
    pub fn is_chinese(s: &str) -> bool {
        Regex::new(r"^[\u4e00-\u9fa5]+$").unwrap().is_match(s)
    }

    /// 验证十六进制颜色值（#RGB, #RRGGBB, #RRGGBBAA）
    pub fn is_hex_color(s: &str) -> bool {
        Regex::new(r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$")
            .unwrap()
            .is_match(s)
    }

    // ── 通用匹配 ──────────────────────────────────────────────

    /// 检查字符串是否匹配正则表达式
    pub fn is_match(pattern: &str, s: &str) -> Result<bool, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.is_match(s))
    }

    /// 查找第一个匹配
    pub fn find(pattern: &str, s: &str) -> Result<Option<String>, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.find(s).map(|m| m.as_str().to_string()))
    }

    /// 查找所有匹配
    pub fn find_all(pattern: &str, s: &str) -> Result<Vec<String>, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.find_iter(s).map(|m| m.as_str().to_string()).collect())
    }

    /// 提取命名捕获组
    pub fn captures(pattern: &str, s: &str) -> Result<Option<Vec<String>>, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.captures(s).map(|caps| {
            (1..caps.len())
                .filter_map(|i| caps.get(i).map(|m| m.as_str().to_string()))
                .collect()
        }))
    }

    /// 提取所有匹配的捕获组
    pub fn captures_all(pattern: &str, s: &str) -> Result<Vec<Vec<String>>, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re
            .captures_iter(s)
            .map(|caps| {
                (1..caps.len())
                    .filter_map(|i| caps.get(i).map(|m| m.as_str().to_string()))
                    .collect()
            })
            .collect())
    }

    /// 替换第一个匹配
    pub fn replace(pattern: &str, s: &str, rep: &str) -> Result<String, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.replace(s, rep).to_string())
    }

    /// 替换所有匹配
    pub fn replace_all(pattern: &str, s: &str, rep: &str) -> Result<String, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.replace_all(s, rep).to_string())
    }

    /// 分割字符串（按正则匹配分割）
    pub fn split(pattern: &str, s: &str) -> Result<Vec<String>, String> {
        let re = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;
        Ok(re.split(s).map(|part| part.to_string()).collect())
    }
}
