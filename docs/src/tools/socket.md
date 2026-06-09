# Socket 长连接

Feature: `socket-binary` / `socket-ws` / `sse`

## 简介

长连接连接管理器，支持按 ID、分组、标签多维度发送消息。适用于 **WebSocket / TCP 长连接 / SSE** 场景，传统 HTTP 模式不可用。

支持三种连接类型：
- **二进制 WS** (`socket-binary` feature)：通过 `afast::Sender` 注册，发送 `Vec<u8>` 二进制数据
- **普通 WS** (`socket-ws` feature)：通过 `afast::WsSender` 注册，支持 text / JSON / binary
- **SSE** (`sse` feature)：通过 `afast::SseSender` 注册，单向服务端推送，支持命名事件

## 配置

无需配置，模块自动初始化。

```rust
use afaster::{Socket, SocketManager};

// 从 AppState 获取管理器
let mgr = &state.socket;

// 创建多个独立的SOCKET
mgr.create("chat").await;
mgr.create("notify").await;

// 获取使用
let chat = mgr.get("chat").await.unwrap();
```

## 核心概念

| 概念 | 说明 |
|------|------|
| **conn_id** | 每个连接的唯一标识（自增 u64），注册时自动分配 |
| **id** | 业务 ID（如 user_id），可选，同一 id 可对应多个 conn_id（多设备） |
| **group** | 分组，一个连接可加入多个分组 |
| **tags** | 键值对标签，一个连接可拥有多个标签 |

## API

### 连接管理

```rust
// 注册二进制 WS 连接（返回 conn_id）
let conn_id = socket.register(Some("user_1001"), &sender, None).await;

// 注册普通 WS 连接（支持 text/json/binary）
let conn_id = socket.register_ws(Some("user_1001"), ws_sender, None).await;

// 注册 SSE 连接（sse feature）
let conn_id = socket.register_sse(Some("user_1001"), sse_sender, None).await;

// 带初始标签注册
let conn_id = socket.register(
    Some("user_1001"),
    &sender,
    Some(vec![("platform".into(), "ios".into())]),
).await;

// 设置业务 ID
socket.set_id(conn_id, "user_1002").await;

// 注销连接（自动清理所有索引）
socket.unregister(conn_id).await;

// 统计
let count = socket.len().await;
let empty = socket.is_empty().await;
```

### 标签管理

```rust
// 设置标签（merge=true 合并，false 替换）
socket.set_tags(conn_id, vec![
    ("lang".into(), "zh".into()),
    ("platform".into(), "ios".into()),
], true).await;

// 追加单个标签
socket.add_tag(conn_id, "version", "2.0").await;

// 移除标签
socket.remove_tag(conn_id, "lang").await;

// 获取所有标签
let tags = socket.get_tags(conn_id).await;
```

### 分组管理

```rust
// 加入分组
socket.join_group(conn_id, "chat_room_1").await;
socket.join_group(conn_id, "vip_users").await;

// 离开分组
socket.leave_group(conn_id, "chat_room_1").await;

// 获取分组成员
let members = socket.group_members("chat_room_1").await;

// 获取连接所属分组
let groups = socket.conn_groups(conn_id).await;
```

### 消息推送

```rust
// 精确发送（按 conn_id）
socket.send_to_conn(conn_id, data).await?;

// 按业务 ID 发送（该用户所有设备）
let errors = socket.send_to_id("user_1001", data).await;

// 按分组发送
let errors = socket.send_to_group("chat_room_1", data).await;

// 广播
let errors = socket.broadcast(data).await;

// 按标签发送（AND 模式：所有标签都匹配）
let errors = socket.send_by_tags(
    &[("lang", "zh"), ("platform", "ios")],
    TagMode::And,
    data,
).await;

// 按标签发送（OR 模式：任一标签匹配）
let errors = socket.send_by_tags(
    &[("lang", "zh"), ("lang", "en")],
    TagMode::Or,
    data,
).await;
```

### 文本 / JSON 推送（`socket-ws` feature）

```rust
// 发送文本（仅普通 WS 连接会收到，二进制连接自动跳过）
socket.send_text_to_conn(conn_id, "hello").await?;

// 发送 JSON
socket.send_json_to_conn(conn_id, &serde_json::json!({"type": "msg", "body": "hi"})).await?;

// 通用 SocketMessage（自动选择 binary/text）
socket.send_to_conn(conn_id, "hello".to_string()).await?;  // Text
socket.send_to_conn(conn_id, b"hello".to_vec()).await?;     // Binary

// 按标签发送文本
socket.send_by_tags(&[("lang", "zh")], TagMode::And, "hello").await;

// 广播文本
socket.broadcast("system notice").await;
```

### SSE 事件推送（`sse` feature）

```rust
use serde_json::json;

// 发送 SSE 事件（指定事件名 + JSON 数据）
socket.send_event_to_conn(conn_id, "tick", &json!({"count": 1})).await?;

// 发送 SSE 事件（传入可序列化结构体）
socket.send_event_json_to_conn(conn_id, "notify", &my_struct).await?;

// 通用 SocketMessage 发送 SSE 事件
socket.send_to_conn(conn_id, ("tick", json!({"count": 1}))).await?;

// 按业务 ID 推送 SSE 事件（该用户所有连接）
socket.send_to_id("user_1001", ("notify", json!({"msg": "hello"}))).await;

// 按分组推送 SSE 事件
socket.send_to_group("chat_room_1", ("message", json!({"text": "hi"}))).await;

// 按标签推送 SSE 事件
socket.send_by_tags(
    &[("lang", "zh")],
    TagMode::And,
    ("update", json!({"data": "..."})),
).await;

// 广播 SSE 事件（SSE 连接收到事件，WS 连接收到 JSON 文本）
socket.broadcast(("system", json!({"notice": "维护通知"}))).await;
```

**跨连接类型兼容**：当 SSE 消息发送到非 SSE 连接时，会自动转换：
- WS 连接：以 JSON 文本形式发送 `data` 字段
- 二进制 WS 连接：以 JSON 二进制形式发送 `data` 字段

### TagMode

```rust
use afaster::TagMode;

TagMode::And  // 所有标签都必须匹配
TagMode::Or   // 任意标签匹配即可
```

## 在 Handler 中使用

### WebSocket Handler

```rust
#[handler(desc("Join chat"))]
async fn join_chat(
    state: State<AppState>,
    data: Data<JoinRequest>,
    sender: Sender,
    mut receiver: Receiver,
) {
    let socket = &state.socket;

    // 注册连接
    let conn_id = socket.register(
        Some(&data.user_id),
        &sender,
        Some(vec![("room".into(), data.room_id.clone())]),
    ).await;

    socket.join_group(conn_id, &data.room_id).await;

    // 监听消息
    while let Some(msg) = receiver.recv().await {
        // 广播到房间
        socket.send_to_group(&data.room_id, msg).await;
    }

    // 连接断开，清理
    socket.unregister(conn_id).await;
}
```

### SSE Handler

```rust
use afast::{SseSender, Query};
use serde::Deserialize;

#[derive(Deserialize)]
struct SseQuery {
    room: Option<String>,
}

#[afast::sse(desc("SSE 事件流"))]
async fn sse_stream(
    state: State<AppState>,
    query: Query<SseQuery>,
    sender: SseSender,
) -> afast::Result<()> {
    let socket = &state.socket;
    let room = query.0.room.unwrap_or_else(|| "default".into());

    // 注册 SSE 连接到 Socket 管理器
    let conn_id = socket.register_sse(
        Some(&room),
        sender.clone(),
        Some(vec![("room".into(), room.clone())]),
    ).await;

    socket.join_group(conn_id, &room).await;

    // 发送连接成功事件
    sender.send_event("connected", &serde_json::json!({"room": room})).await?;

    // 循环推送（实际场景可从 Redis/DB 读取数据）
    let mut count = 0u64;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        count += 1;
        let sent = sender.send_event("tick", &serde_json::json!({
            "count": count,
            "room": room,
        })).await;
        if sent.is_err() { break; }
    }

    // 连接断开，清理
    socket.unregister(conn_id).await;
    Ok(())
}
```

## 注意事项

- `send_*` 方法返回 `Vec<(u64, String)>` 表示失败的连接及其错误信息
- 注销连接时自动清理分组、ID 索引，无需手动维护
- 同一 user_id 可以有多个 conn_id（多设备在线）
- 标签使用键值对 `HashMap<String, String>`，支持精确匹配
- 多个独立推送场景（聊天、通知、游戏）使用 `SocketManager` 管理多个 `Socket` 实例
- SSE 是单向推送（服务端→客户端），不支持接收客户端消息
- SSE 连接断开由客户端触发，服务端通过 `send` 返回 `Err` 感知

## SocketManager

管理多个独立的 `Socket` 实例，按名称隔离。

```rust
use afaster::SocketManager;

let mgr = SocketManager::new();

// 创建
mgr.create("chat").await;
mgr.create("notify").await;

// 获取
let chat = mgr.get("chat").await.unwrap();
chat.register(Some("user_1"), &sender, None).await;

// 判断是否存在
let exists = mgr.contains("chat").await;

// 列出所有
let names = mgr.list().await;  // ["chat", "notify"]

// 移除
mgr.remove("notify").await;

// 数量
let n = mgr.len().await;
```
