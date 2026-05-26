// marianne-server/src/routes/mod.rs

pub mod chat;
pub mod documents;
pub mod history;
pub mod profile;

use crate::state::ServerState;
use axum::{routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn build_router(state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", api_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn api_routes() -> Router<ServerState> {
    Router::new()
        .route("/chat", axum::routing::post(chat::chat_handler))
        .route(
            "/history/:conversation_id",
            get(history::get_history_handler),
        )
        .route("/profile", get(profile::get_profile_handler))
        .route(
            "/profile",
            axum::routing::put(profile::update_profile_handler),
        )
        .route(
            "/documents/extract",
            axum::routing::post(documents::extract_handler),
        )
}

async fn health() -> &'static str {
    "ok"
}
