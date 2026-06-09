//! Binary 长连接聊天（binary protocol）
//!
//! 使用 `#[handler]` 宏 + `Sender`/`Receiver`，走 afast 二进制协议。

use crate::model::*;
use afast::{Ctx, Data, Receiver, Sender, State, handler};
use afaster::trace::TraceContext;

/// Binary 聊天回显
///
/// 客户端通过 binary 协议连接，发送 `ChatJoin` 加入，
/// 后续发送的消息原样回显。
#[handler(desc("Binary 聊天回显"))]
pub async fn bin_chat_echo(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    Data(join): Data<ChatJoin>,
    mut receiver: Receiver,
    sender: Sender,
) {
    // 发送欢迎消息
    let welcome = ChatMessage {
        room: join.room.clone(),
        from: "system".into(),
        content: format!("{} 加入了 {}", join.nickname, join.room),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    let _ = sender
        .send(serde_json::to_vec(&welcome).unwrap_or_default())
        .await;

    // 消息循环：接收并回显
    while let Some(data) = receiver.recv().await {
        // 尝试解析为 ChatMessage
        {
            let _s = state
                .tracing
                .span_with(&trace, "bin_chat_message", "Long 消息");
            if let Ok(mut msg) = serde_json::from_slice::<ChatMessage>(&data) {
                msg.from = join.nickname.clone();
                msg.room = join.room.clone();
                msg.timestamp = chrono::Utc::now().timestamp_millis();
                let _ = sender
                    .send(serde_json::to_vec(&msg).unwrap_or_default())
                    .await;
            } else {
                // 原样回显
                let _ = sender.send(data).await;
            }
        }
    }

    // 离开消息
    let bye = ChatMessage {
        room: join.room.clone(),
        from: "system".into(),
        content: format!("{} 离开了 {}", join.nickname, join.room),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };
    let _ = sender
        .send(serde_json::to_vec(&bye).unwrap_or_default())
        .await;
}

/// Binary 广播通知
///
/// 客户端连接后，服务端定时推送系统通知。
#[handler(desc("Binary 系统通知推送"))]
pub async fn bin_notify(
    State(state): State<afaster::AppState>,
    Ctx(trace): Ctx<TraceContext>,
    mut receiver: Receiver,
    sender: Sender,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut count = 0u64;

    loop {
        tokio::select! {
            _ = interval.tick() => {
                count += 1;
                let msg = ChatMessage {
                    room: "system".into(),
                    from: "server".into(),
                    content: format!("心跳 #{} — 服务运行正常", count),
                    timestamp: chrono::Utc::now().timestamp_millis(),
                };
                if sender.send(serde_json::to_vec(&msg).unwrap_or_default()).await.is_err() {
                    break;
                }
            }
            data = receiver.recv() => {
                match data {
                    Some(bytes) => {
                        // 客户端发来的消息原样回显
                        {
                            let _s = state.tracing.span_with(&trace, "bin_chat_message", "Long 消息");
                            let _ = sender.send(bytes).await;
                        }
                    }
                    None => break, // 连接关闭
                }
            }
        }
    }
}

pub fn build_service() -> afast::Service {
    afast::service!("bin_chat", "Binary 聊天" => {
        h(bin_chat_echo),
        h(bin_notify),
    })
}
