// marianne-server/src/routes/history.rs
use crate::state::ServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use marianne_core::prompts::system::ConversationTurn;

pub async fn get_history_handler(
    State(server): State<ServerState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<ConversationTurn>>, StatusCode> {
    server
        .core
        .history
        .get_conversation(&conversation_id)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::warn!("history error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}
