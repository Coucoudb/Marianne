// marianne-server/src/routes/chat.rs
// POST /api/v1/chat — réponse en SSE (Server-Sent Events).
//
// Le client consomme le flux SSE avec EventSource :
//   const es = new EventSource('/api/v1/chat');
//   es.addEventListener('stream-token', e => append(e.data));
//   es.addEventListener('generation-done', e => finalize(JSON.parse(e.data)));

use crate::state::ServerState;
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    Json,
};
use marianne_core::chat::{process_chat, ChatEvent, ChatRequest};
use serde_json::json;
use std::convert::Infallible;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

pub async fn chat_handler(
    State(server): State<ServerState>,
    Json(request): Json<ChatRequest>,
) -> axum::response::Response {
    use axum::response::IntoResponse;

    let (tx, rx) = mpsc::channel::<ChatEvent>(64);
    let core = (*server.core).clone();

    // Spawn the core pipeline — it writes ChatEvents to tx
    tokio::spawn(async move {
        if let Err(e) = process_chat(core, request, tx).await {
            tracing::warn!("process_chat error: {}", e);
        }
    });

    // Map ChatEvent → SSE Event
    let stream = ReceiverStream::new(rx).map(|event| {
        let sse_event = match &event {
            ChatEvent::StreamToken { token, conversation_id } => Event::default()
                .event("stream-token")
                .data(json!({ "token": token, "conversation_id": conversation_id }).to_string()),

            ChatEvent::GenerationDone { .. } => Event::default()
                .event("generation-done")
                .data(serde_json::to_string(&event).unwrap_or_default()),

            ChatEvent::ConfidenceInfo { .. } => Event::default()
                .event("confidence-info")
                .data(serde_json::to_string(&event).unwrap_or_default()),

            ChatEvent::ContradictionWarning { .. } => Event::default()
                .event("contradiction-warning")
                .data(serde_json::to_string(&event).unwrap_or_default()),

            ChatEvent::WebSearchStatus { .. } => Event::default()
                .event("web-search-status")
                .data(serde_json::to_string(&event).unwrap_or_default()),

            ChatEvent::OfflineMode { .. } => Event::default()
                .event("offline-mode")
                .data(serde_json::to_string(&event).unwrap_or_default()),
        };
        Ok::<Event, Infallible>(sse_event)
    });

    Sse::new(stream).into_response()
}
