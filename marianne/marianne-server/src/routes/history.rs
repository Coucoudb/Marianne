// marianne-server/src/routes/history.rs
use crate::middleware::auth::UserId;
use crate::state::ServerState;
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use marianne_core::history::sqlite::{ConversationMessage, ConversationSummary};
use marianne_core::prompts::system::ConversationTurn;

/// GET /api/v1/history/conversations — Liste toutes les conversations
pub async fn list_conversations_handler(
    State(server): State<ServerState>,
    Extension(user): Extension<UserId>,
) -> Result<Json<Vec<ConversationSummary>>, StatusCode> {
    server
        .core
        .history
        .list_conversations(&user.0)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!("list conversations error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// GET /api/v1/history/:conversation_id — Historique d'une conversation (format client)
pub async fn get_history_handler(
    State(server): State<ServerState>,
    Extension(user): Extension<UserId>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<ConversationMessage>>, StatusCode> {
    server
        .core
        .history
        .get_conversation_messages(&conversation_id, &user.0)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!("history error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// GET /api/v1/history/:conversation_id/turns — Historique au format ConversationTurn (interne)
#[allow(dead_code)]
pub async fn get_history_turns_handler(
    State(server): State<ServerState>,
    Extension(user): Extension<UserId>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<ConversationTurn>>, StatusCode> {
    server
        .core
        .history
        .get_conversation(&conversation_id, &user.0)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!("history turns error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
