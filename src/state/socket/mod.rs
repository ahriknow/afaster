use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "sse")]
use serde_json;
use tokio::sync::Mutex;

// ═══════════════════════════════════════════════════════════════
//  连接推送中心
// ═══════════════════════════════════════════════════════════════

/// Tag 过滤模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagMode {
    /// 所有标签都必须匹配
    And,
    /// 任意标签匹配即可
    Or,
}

/// 消息内容
#[derive(Debug, Clone)]
pub enum SocketMessage {
    /// 二进制数据
    Binary(Vec<u8>),
    /// 文本消息（仅 ordinary WS 连接支持）
    Text(String),
    /// SSE 事件（仅 SSE 连接支持）
    #[cfg(feature = "sse")]
    Sse {
        /// 事件名称
        event: String,
        /// 事件数据（JSON）
        data: serde_json::Value,
    },
}

impl From<Vec<u8>> for SocketMessage {
    fn from(data: Vec<u8>) -> Self {
        Self::Binary(data)
    }
}

impl From<String> for SocketMessage {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for SocketMessage {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

#[cfg(feature = "sse")]
impl From<(&str, serde_json::Value)> for SocketMessage {
    fn from((event, data): (&str, serde_json::Value)) -> Self {
        Self::Sse {
            event: event.to_string(),
            data,
        }
    }
}

#[cfg(feature = "sse")]
impl From<(String, serde_json::Value)> for SocketMessage {
    fn from((event, data): (String, serde_json::Value)) -> Self {
        Self::Sse { event, data }
    }
}

/// 发送通道类型
#[derive(Clone)]
enum SenderKind {
    /// afast 二进制协议 WS (mpsc channel)
    #[cfg(feature = "socket-binary")]
    Binary(tokio::sync::mpsc::Sender<Vec<u8>>),
    /// afast 普通 WS (WsSender，支持 text/json/binary)
    #[cfg(feature = "socket-ws")]
    Ws(afast::WsSender),
    /// SSE (SseSender，单向服务端推送)
    #[cfg(feature = "sse")]
    Sse(afast::SseSender),
}

impl SenderKind {
    /// 发送消息（自动选择 binary 或 text）
    async fn send(&self, msg: &SocketMessage) -> Result<(), String> {
        match (self, msg) {
            #[cfg(feature = "socket-binary")]
            (SenderKind::Binary(tx), SocketMessage::Binary(data)) => tx
                .send(data.clone())
                .await
                .map_err(|_| "channel closed".to_string()),
            #[cfg(feature = "socket-binary")]
            (SenderKind::Binary(_), SocketMessage::Text(_)) => {
                Err("binary sender cannot send text".to_string())
            }
            #[cfg(all(feature = "socket-binary", feature = "sse"))]
            (SenderKind::Binary(tx), SocketMessage::Sse { data, .. }) => {
                let json = serde_json::to_vec(data).unwrap_or_default();
                tx.send(json)
                    .await
                    .map_err(|_| "channel closed".to_string())
            }
            #[cfg(feature = "socket-ws")]
            (SenderKind::Ws(ws), SocketMessage::Binary(data)) => ws
                .send_binary(data.clone())
                .await
                .map_err(|e| e.to_string()),
            #[cfg(feature = "socket-ws")]
            (SenderKind::Ws(ws), SocketMessage::Text(text)) => {
                ws.send_text(text).await.map_err(|e| e.to_string())
            }
            #[cfg(all(feature = "socket-ws", feature = "sse"))]
            (SenderKind::Ws(ws), SocketMessage::Sse { data, .. }) => {
                ws.send_json(data).await.map_err(|e| e.to_string())
            }
            #[cfg(feature = "sse")]
            (SenderKind::Sse(sse), SocketMessage::Binary(data)) => {
                let json_str = serde_json::to_string(data).unwrap_or_default();
                sse.send(&serde_json::Value::String(json_str))
                    .await
                    .map_err(|e| e.to_string())
            }
            #[cfg(feature = "sse")]
            (SenderKind::Sse(sse), SocketMessage::Text(text)) => sse
                .send(&serde_json::Value::String(text.clone()))
                .await
                .map_err(|e| e.to_string()),
            #[cfg(feature = "sse")]
            (SenderKind::Sse(sse), SocketMessage::Sse { event, data }) => {
                sse.send_event(event, data).await.map_err(|e| e.to_string())
            }
        }
    }

    /// 仅发送二进制数据
    async fn send_binary(&self, data: &[u8]) -> Result<(), String> {
        match self {
            #[cfg(feature = "socket-binary")]
            SenderKind::Binary(tx) => tx
                .send(data.to_vec())
                .await
                .map_err(|_| "channel closed".to_string()),
            #[cfg(feature = "socket-ws")]
            SenderKind::Ws(ws) => ws
                .send_binary(data.to_vec())
                .await
                .map_err(|e| e.to_string()),
            #[cfg(feature = "sse")]
            SenderKind::Sse(_) => Err("sse sender cannot send binary".to_string()),
        }
    }

    /// 仅发送文本（仅 ordinary WS 支持）
    #[cfg(feature = "socket-ws")]
    async fn send_text(&self, text: &str) -> Result<(), String> {
        match self {
            #[cfg(feature = "socket-binary")]
            SenderKind::Binary(_) => Err("binary sender cannot send text".to_string()),
            SenderKind::Ws(ws) => ws.send_text(text).await.map_err(|e| e.to_string()),
            #[cfg(feature = "sse")]
            SenderKind::Sse(_) => Err("sse sender cannot send text".to_string()),
        }
    }

    /// 发送 JSON（仅 ordinary WS 支持）
    #[cfg(feature = "socket-ws")]
    async fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<(), String> {
        match self {
            #[cfg(feature = "socket-binary")]
            SenderKind::Binary(_) => Err("binary sender cannot send json".to_string()),
            SenderKind::Ws(ws) => ws.send_json(value).await.map_err(|e| e.to_string()),
            #[cfg(feature = "sse")]
            SenderKind::Sse(_) => Err("sse sender cannot send json".to_string()),
        }
    }

    /// 发送 SSE 事件（仅 SSE 连接支持）
    #[cfg(feature = "sse")]
    async fn send_event(&self, event: &str, data: &serde_json::Value) -> Result<(), String> {
        match self {
            SenderKind::Sse(sse) => sse.send_event(event, data).await.map_err(|e| e.to_string()),
            _ => Err("send_event only supported for SSE connections".to_string()),
        }
    }
}

/// 单个连接信息
#[derive(Clone)]
struct Conn {
    /// 业务 ID（如 user_id），可选
    id: Option<String>,
    /// 发送通道
    tx: SenderKind,
    /// 键值对标签
    tags: HashMap<String, String>,
}

/// 连接推送中心
///
/// 管理所有长连接，支持按 ID、分组、标签多维度推送消息。
///
/// 支持三种连接类型：
/// - **二进制 WS** (`afast::Sender`)：通过 `register` 注册
/// - **普通 WS** (`afast::WsSender`)：通过 `register_ws` 注册，支持 text/json/binary
/// - **SSE** (`afast::SseSender`)：通过 `register_sse` 注册，单向服务端推送
///
/// ```no_run
/// use afaster::socket::{Socket, SocketMessage};
///
/// let socket = Socket::new();
///
/// // 二进制 WS 连接
/// let conn_id = socket.register(None::<&str>, &sender, None).await;
///
/// // 普通 WS 连接
/// let conn_id = socket.register_ws(None::<&str>, ws_sender, None).await;
///
/// // 设置标签和分组
/// socket.set_id(conn_id, "user_1001").await;
/// socket.set_tags(conn_id, [("lang", "zh"), ("platform", "ios")], false).await;
/// socket.join_group(conn_id, "chat_room_1").await;
///
/// // 推送二进制消息
/// socket.send_by_tags(&[("lang", "zh")], socket::TagMode::And, b"hello".to_vec()).await;
///
/// // 推送文本消息（仅普通 WS 连接会收到）
/// socket.send_by_tags_text(&[("lang", "zh")], socket::TagMode::And, "hello").await;
/// ```
#[derive(Clone)]
pub struct Socket {
    inner: Arc<SocketInner>,
}

struct SocketInner {
    /// conn_id → 连接信息
    connections: Mutex<HashMap<u64, Conn>>,
    /// id → conn_id 列表（同一用户可能多设备）
    id_index: Mutex<HashMap<String, Vec<u64>>>,
    /// group → conn_id 集合
    groups: Mutex<HashMap<String, HashSet<u64>>>,
    /// 自增连接 ID
    next_id: AtomicU64,
}

// ──────────────────────────────────────────────────────────────
//  构造
// ──────────────────────────────────────────────────────────────

impl Default for Socket {
    fn default() -> Self {
        Self::new()
    }
}

impl Socket {
    /// 创建新的推送中心
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SocketInner {
                connections: Mutex::new(HashMap::new()),
                id_index: Mutex::new(HashMap::new()),
                groups: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    /// 当前活跃连接数
    pub async fn len(&self) -> usize {
        self.inner.connections.lock().await.len()
    }

    /// 是否无连接
    pub async fn is_empty(&self) -> bool {
        self.inner.connections.lock().await.is_empty()
    }
}

// ──────────────────────────────────────────────────────────────
//  连接注册 / 注销
// ──────────────────────────────────────────────────────────────

impl Socket {
    /// 注册一个新的二进制 WS 连接
    ///
    /// - `id` — 业务 ID（如 user_id），可选
    /// - `sender` — afast 的 Sender（会 clone 内部 channel）
    /// - `tags` — 初始标签，可选
    ///
    /// 返回分配的 `conn_id`
    #[cfg(feature = "socket-binary")]
    pub async fn register<I, T>(
        &self,
        id: Option<I>,
        sender: &afast::Sender,
        tags: Option<T>,
    ) -> u64
    where
        I: Display,
        T: IntoIterator<Item = (String, String)>,
    {
        let conn_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let id_str = id.map(|i| i.to_string());

        let conn = Conn {
            id: id_str.clone(),
            tx: SenderKind::Binary(sender.tx().clone()),
            tags: tags.map(|t| t.into_iter().collect()).unwrap_or_default(),
        };

        self.inner.connections.lock().await.insert(conn_id, conn);

        // 建立 id 索引
        if let Some(ref uid) = id_str {
            self.inner
                .id_index
                .lock()
                .await
                .entry(uid.clone())
                .or_default()
                .push(conn_id);
        }

        conn_id
    }

    /// 注册一个新的普通 WS 连接（支持 text/json/binary）
    ///
    /// - `id` — 业务 ID（如 user_id），可选
    /// - `sender` — afast 的 WsSender
    /// - `tags` — 初始标签，可选
    ///
    /// 返回分配的 `conn_id`
    #[cfg(feature = "socket-ws")]
    pub async fn register_ws<I, T>(
        &self,
        id: Option<I>,
        sender: afast::WsSender,
        tags: Option<T>,
    ) -> u64
    where
        I: Display,
        T: IntoIterator<Item = (String, String)>,
    {
        let conn_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let id_str = id.map(|i| i.to_string());

        let conn = Conn {
            id: id_str.clone(),
            tx: SenderKind::Ws(sender),
            tags: tags.map(|t| t.into_iter().collect()).unwrap_or_default(),
        };

        self.inner.connections.lock().await.insert(conn_id, conn);

        // 建立 id 索引
        if let Some(ref uid) = id_str {
            self.inner
                .id_index
                .lock()
                .await
                .entry(uid.clone())
                .or_default()
                .push(conn_id);
        }

        conn_id
    }

    /// 注册一个新的 SSE 连接
    ///
    /// - `id` — 业务 ID（如 user_id），可选
    /// - `sender` — afast 的 SseSender
    /// - `tags` — 初始标签，可选
    ///
    /// 返回分配的 `conn_id`
    #[cfg(feature = "sse")]
    pub async fn register_sse<I, T>(
        &self,
        id: Option<I>,
        sender: afast::SseSender,
        tags: Option<T>,
    ) -> u64
    where
        I: Display,
        T: IntoIterator<Item = (String, String)>,
    {
        let conn_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let id_str = id.map(|i| i.to_string());

        let conn = Conn {
            id: id_str.clone(),
            tx: SenderKind::Sse(sender),
            tags: tags.map(|t| t.into_iter().collect()).unwrap_or_default(),
        };

        self.inner.connections.lock().await.insert(conn_id, conn);

        // 建立 id 索引
        if let Some(ref uid) = id_str {
            self.inner
                .id_index
                .lock()
                .await
                .entry(uid.clone())
                .or_default()
                .push(conn_id);
        }

        conn_id
    }

    /// 注销连接，自动清理所有索引
    pub async fn unregister(&self, conn_id: u64) {
        let conn = self.inner.connections.lock().await.remove(&conn_id);

        if let Some(conn) = conn {
            // 清理 id 索引
            if let Some(ref uid) = conn.id {
                let mut idx = self.inner.id_index.lock().await;
                if let Some(list) = idx.get_mut(uid) {
                    list.retain(|&id| id != conn_id);
                    if list.is_empty() {
                        idx.remove(uid);
                    }
                }
            }

            // 清理分组索引
            let mut groups = self.inner.groups.lock().await;
            for members in groups.values_mut() {
                members.remove(&conn_id);
            }
            groups.retain(|_, members| !members.is_empty());
        }
    }

    /// 设置或更新业务 ID
    pub async fn set_id<I: Display>(&self, conn_id: u64, id: I) {
        let id_str = id.to_string();
        let mut conns = self.inner.connections.lock().await;

        if let Some(conn) = conns.get_mut(&conn_id) {
            // 移除旧索引
            if let Some(ref old_id) = conn.id {
                let mut idx = self.inner.id_index.lock().await;
                if let Some(list) = idx.get_mut(old_id) {
                    list.retain(|&id| id != conn_id);
                    if list.is_empty() {
                        idx.remove(old_id);
                    }
                }
            }

            // 建立新索引
            self.inner
                .id_index
                .lock()
                .await
                .entry(id_str.clone())
                .or_default()
                .push(conn_id);

            conn.id = Some(id_str);
        }
    }
}

// ──────────────────────────────────────────────────────────────
//  标签管理
// ──────────────────────────────────────────────────────────────

impl Socket {
    /// 设置标签
    ///
    /// - `merge = true`  → 与已有标签合并（覆盖同 key）
    /// - `merge = false` → 替换全部标签
    pub async fn set_tags<I>(&self, conn_id: u64, tags: I, merge: bool)
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get_mut(&conn_id) {
            if merge {
                conn.tags.extend(tags);
            } else {
                conn.tags = tags.into_iter().collect();
            }
        }
    }

    /// 追加单个标签
    pub async fn add_tag<K: Display, V: Display>(&self, conn_id: u64, key: K, value: V) {
        let mut conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get_mut(&conn_id) {
            conn.tags.insert(key.to_string(), value.to_string());
        }
    }

    /// 移除标签
    pub async fn remove_tag<K: Display>(&self, conn_id: u64, key: K) {
        let mut conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get_mut(&conn_id) {
            conn.tags.remove(&key.to_string());
        }
    }

    /// 获取连接的所有标签
    pub async fn get_tags(&self, conn_id: u64) -> Option<HashMap<String, String>> {
        self.inner
            .connections
            .lock()
            .await
            .get(&conn_id)
            .map(|c| c.tags.clone())
    }
}

// ──────────────────────────────────────────────────────────────
//  分组管理
// ──────────────────────────────────────────────────────────────

impl Socket {
    /// 加入分组
    pub async fn join_group<G: Display>(&self, conn_id: u64, group: G) {
        self.inner
            .groups
            .lock()
            .await
            .entry(group.to_string())
            .or_default()
            .insert(conn_id);
    }

    /// 离开分组
    pub async fn leave_group<G: Display>(&self, conn_id: u64, group: G) {
        let mut groups = self.inner.groups.lock().await;
        if let Some(members) = groups.get_mut(&group.to_string()) {
            members.remove(&conn_id);
            if members.is_empty() {
                groups.remove(&group.to_string());
            }
        }
    }

    /// 获取分组内的所有 conn_id
    pub async fn group_members<G: Display>(&self, group: G) -> Vec<u64> {
        self.inner
            .groups
            .lock()
            .await
            .get(&group.to_string())
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// 获取连接所属的所有分组
    pub async fn conn_groups(&self, conn_id: u64) -> Vec<String> {
        self.inner
            .groups
            .lock()
            .await
            .iter()
            .filter(|(_, members)| members.contains(&conn_id))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

// ──────────────────────────────────────────────────────────────
//  消息推送
// ──────────────────────────────────────────────────────────────

impl Socket {
    /// 向指定连接发送消息
    pub async fn send_to_conn(
        &self,
        conn_id: u64,
        msg: impl Into<SocketMessage>,
    ) -> Result<(), String> {
        let conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get(&conn_id) {
            conn.tx.send(&msg.into()).await
        } else {
            Err(format!("conn {} not found", conn_id))
        }
    }

    /// 向指定连接发送二进制数据
    pub async fn send_binary_to_conn(&self, conn_id: u64, data: &[u8]) -> Result<(), String> {
        let conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get(&conn_id) {
            conn.tx.send_binary(data).await
        } else {
            Err(format!("conn {} not found", conn_id))
        }
    }

    /// 向指定连接发送文本（仅普通 WS 连接支持）
    #[cfg(feature = "socket-ws")]
    pub async fn send_text_to_conn(&self, conn_id: u64, text: &str) -> Result<(), String> {
        let conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get(&conn_id) {
            conn.tx.send_text(text).await
        } else {
            Err(format!("conn {} not found", conn_id))
        }
    }

    /// 向指定连接发送 JSON（仅普通 WS 连接支持）
    #[cfg(feature = "socket-ws")]
    pub async fn send_json_to_conn<T: serde::Serialize>(
        &self,
        conn_id: u64,
        value: &T,
    ) -> Result<(), String> {
        let conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get(&conn_id) {
            conn.tx.send_json(value).await
        } else {
            Err(format!("conn {} not found", conn_id))
        }
    }

    /// 向指定连接发送 SSE 事件（仅 SSE 连接支持）
    #[cfg(feature = "sse")]
    pub async fn send_event_to_conn(
        &self,
        conn_id: u64,
        event: &str,
        data: &serde_json::Value,
    ) -> Result<(), String> {
        let conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get(&conn_id) {
            conn.tx.send_event(event, data).await
        } else {
            Err(format!("conn {} not found", conn_id))
        }
    }

    /// 向指定连接发送 SSE 事件（传入可序列化的数据）
    #[cfg(feature = "sse")]
    pub async fn send_event_json_to_conn<T: serde::Serialize>(
        &self,
        conn_id: u64,
        event: &str,
        data: &T,
    ) -> Result<(), String> {
        let value = serde_json::to_value(data).map_err(|e| e.to_string())?;
        let conns = self.inner.connections.lock().await;
        if let Some(conn) = conns.get(&conn_id) {
            conn.tx.send_event(event, &value).await
        } else {
            Err(format!("conn {} not found", conn_id))
        }
    }

    /// 向指定业务 ID 的所有连接发送消息
    pub async fn send_to_id<I: Display>(
        &self,
        id: I,
        msg: impl Into<SocketMessage>,
    ) -> Vec<(u64, String)> {
        let msg = msg.into();
        let conn_ids: Vec<u64> = {
            let idx = self.inner.id_index.lock().await;
            idx.get(&id.to_string()).cloned().unwrap_or_default()
        };

        let mut errors = Vec::new();
        let conns = self.inner.connections.lock().await;
        for cid in conn_ids {
            if let Some(conn) = conns.get(&cid)
                && let Err(e) = conn.tx.send(&msg).await
            {
                errors.push((cid, e));
            }
        }
        errors
    }

    /// 向分组内所有连接发送消息
    pub async fn send_to_group<G: Display>(
        &self,
        group: G,
        msg: impl Into<SocketMessage>,
    ) -> Vec<(u64, String)> {
        let msg = msg.into();
        let member_ids: Vec<u64> = {
            let groups = self.inner.groups.lock().await;
            groups
                .get(&group.to_string())
                .map(|s| s.iter().copied().collect())
                .unwrap_or_default()
        };

        let mut errors = Vec::new();
        let conns = self.inner.connections.lock().await;
        for cid in member_ids {
            if let Some(conn) = conns.get(&cid)
                && let Err(e) = conn.tx.send(&msg).await
            {
                errors.push((cid, e));
            }
        }
        errors
    }

    /// 广播给所有连接
    pub async fn broadcast(&self, msg: impl Into<SocketMessage>) -> Vec<(u64, String)> {
        let msg = msg.into();
        let mut errors = Vec::new();
        let conns = self.inner.connections.lock().await;
        for (cid, conn) in conns.iter() {
            if let Err(e) = conn.tx.send(&msg).await {
                errors.push((*cid, e));
            }
        }
        errors
    }

    /// 按标签过滤并发送
    ///
    /// - `tags` — 要匹配的键值对
    /// - `mode` — `And` 全部匹配 / `Or` 任一匹配
    pub async fn send_by_tags(
        &self,
        tags: &[(&str, &str)],
        mode: TagMode,
        msg: impl Into<SocketMessage>,
    ) -> Vec<(u64, String)> {
        let msg = msg.into();
        let mut errors = Vec::new();
        let conns = self.inner.connections.lock().await;

        for (cid, conn) in conns.iter() {
            let matched = match mode {
                TagMode::And => tags
                    .iter()
                    .all(|(k, v)| conn.tags.get(*k).is_some_and(|cv| cv == *v)),
                TagMode::Or => tags
                    .iter()
                    .any(|(k, v)| conn.tags.get(*k).is_some_and(|cv| cv == *v)),
            };

            if matched && let Err(e) = conn.tx.send(&msg).await {
                errors.push((*cid, e));
            }
        }

        errors
    }
}

// ═══════════════════════════════════════════════════════════════
//  推送中心管理器
// ═══════════════════════════════════════════════════════════════

/// 多推送中心管理器
///
/// 一个项目可能有多个独立的推送场景（如聊天、通知、游戏），
/// `SocketManager` 按名称管理多个独立的 `Socket` 实例。
///
/// ```no_run
/// use afaster::socket::SocketManager;
///
/// let mgr = SocketManager::new();
/// mgr.create("chat");
/// mgr.create("notify");
///
/// mgr.get("chat").unwrap().register(None::<&str>, &sender, None).await;
/// mgr.get("notify").unwrap().broadcast(b"hello".to_vec()).await;
/// ```
#[derive(Clone)]
pub struct SocketManager {
    instances: Arc<Mutex<HashMap<String, Socket>>>,
}

impl Default for SocketManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketManager {
    /// 创建空的管理器
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 创建一个命名推送中心（已存在则忽略）
    pub async fn create<N: Display>(&self, name: N) {
        self.instances
            .lock()
            .await
            .entry(name.to_string())
            .or_insert_with(Socket::new);
    }

    /// 创建一个命名推送中心并返回引用
    pub async fn create_and_get<N: Display>(&self, name: N) -> Socket {
        let mut map = self.instances.lock().await;
        map.entry(name.to_string())
            .or_insert_with(Socket::new)
            .clone()
    }

    /// 获取命名推送中心
    pub async fn get<N: Display>(&self, name: N) -> Option<Socket> {
        self.instances.lock().await.get(&name.to_string()).cloned()
    }

    /// 移除并返回命名推送中心
    pub async fn remove<N: Display>(&self, name: N) -> Option<Socket> {
        self.instances.lock().await.remove(&name.to_string())
    }

    /// 是否存在指定名称
    pub async fn contains<N: Display>(&self, name: N) -> bool {
        self.instances.lock().await.contains_key(&name.to_string())
    }

    /// 所有推送中心名称
    pub async fn list(&self) -> Vec<String> {
        self.instances.lock().await.keys().cloned().collect()
    }

    /// 推送中心数量
    pub async fn len(&self) -> usize {
        self.instances.lock().await.len()
    }

    /// 是否为空
    pub async fn is_empty(&self) -> bool {
        self.instances.lock().await.is_empty()
    }
}
