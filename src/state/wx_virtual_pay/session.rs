use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

// ═══════════════════════════════════════════════════════════════
//  缓存条目
// ═══════════════════════════════════════════════════════════════

#[derive(Clone)]
struct CachedSession {
    session_key: String,
    updated_at: Instant,
}

// ═══════════════════════════════════════════════════════════════
//  微信小程序登录响应
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct WxMiniLoginResponse {
    pub openid: String,
    pub session_key: String,
    pub unionid: Option<String>,
    pub errcode: Option<i32>,
    pub errmsg: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
//  Session 管理器
// ═══════════════════════════════════════════════════════════════

/// 微信 session_key 管理器
///
/// 按 `(appid, openid)` 存储 session_key，支持多个小程序共存。
/// 内部使用 `Arc<RwLock<HashMap>>` 实现线程安全的跨请求共享。
#[derive(Clone)]
pub struct WxSessionManager {
    sessions: Arc<RwLock<HashMap<String, CachedSession>>>,
}

impl WxSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 存入 session_key
    ///
    /// `key` 格式建议为 `"appid:openid"`
    pub fn set(&self, key: &str, session_key: &str) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(
            key.to_string(),
            CachedSession {
                session_key: session_key.to_string(),
                updated_at: Instant::now(),
            },
        );
    }

    /// 取出 session_key
    pub fn get(&self, key: &str) -> Option<String> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(key).map(|s| s.session_key.clone())
    }

    /// 删除指定 session
    pub fn remove(&self, key: &str) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(key);
    }

    /// 是否存在
    pub fn contains(&self, key: &str) -> bool {
        let sessions = self.sessions.read().unwrap();
        sessions.contains_key(key)
    }

    /// 获取条目存入时间
    pub fn updated_at(&self, key: &str) -> Option<Instant> {
        let sessions = self.sessions.read().unwrap();
        sessions.get(key).map(|s| s.updated_at)
    }

    /// 清除所有 session
    pub fn clear(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.clear();
    }
}

impl Default for WxSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
