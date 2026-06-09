//! 普通 WebSocket 聊天（ordinary-ws）
//!
//! 使用 `#[afast::ws]` 宏，支持标准 WebSocket 文本/JSON 帧。

use crate::model::*;
use afast::{Ctx, State, WsMessage, WsReceiver, WsSender};
use afaster::trace::TraceContext;

/// 普通 WebSocket 聊天
///
/// 客户端连接后发送 JSON 消息，服务端广播给同一房间的所有连接。
#[afast::ws(desc("WebSocket 聊天"))]
pub async fn ws_chat(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    sender: WsSender,
    mut receiver: WsReceiver,
) -> afast::Result<()> {
    // 等待第一条消息获取加入信息
    let first = receiver.recv().await;
    let join: ChatJoin = match first {
        Some(WsMessage::Text(text)) => serde_json::from_str(&text)
            .map_err(|e| afaster::Error::custom(40001, format!("无效的加入消息: {}", e)))?,
        _ => return Err(afaster::Error::custom(40001, "期望 JSON 加入消息").into()),
    };

    // 发送欢迎消息
    let welcome = serde_json::json!({
        "type": "system",
        "content": format!("{} 加入了 {}", join.nickname, join.room),
        "timestamp": chrono::Utc::now().timestamp_millis()
    });
    if let Err(e) = sender.send_json(&welcome).await {
        eprintln!("[ws_chat] welcome send error: {}", e);
    }

    // 消息循环
    loop {
        match receiver.recv().await {
            Some(WsMessage::Text(text)) => {
                let _s = state
                    .tracing
                    .span_root_with(&trace, "ws_chat_message", "WebSocket 消息");
                let msg = ChatMessage {
                    room: join.room.clone(),
                    from: join.nickname.clone(),
                    content: text,
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };
                if let Err(e) = sender.send_json(&msg).await {
                    eprintln!("[ws_chat] echo send error: {}", e);
                    break;
                }
            }
            Some(WsMessage::Close(_)) | None => break,
            _ => {} // 忽略 Ping/Pong/Binary
        }
    }

    // 离开消息
    let bye = serde_json::json!({
        "type": "system",
        "content": format!("{} 离开了 {}", join.nickname, join.room),
        "timestamp": chrono::Utc::now().timestamp_millis()
    });
    let _ = sender.send_json(&bye).await;

    Ok(())
}

pub fn build_service(url: &'static str) -> afast::Service {
    afast::service!("ws_chat", "WebSocket 聊天" => {
        ws(url, ws_chat),
    })
}
