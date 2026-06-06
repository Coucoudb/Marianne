// marianne-server/src/routes/mod.rs

pub mod chat;
pub mod documents;
pub mod history;
pub mod models;
pub mod profile;
pub mod system;
pub mod workspace;

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
        .route("/system/info", get(system::get_system_info))
        .route("/models/status", get(models::get_models_status))
        .route("/models/download", axum::routing::post(models::download_model))
        .route("/models/load", axum::routing::post(models::load_model))
        .route("/models/setup", axum::routing::post(models::setup_model))
        .route("/models/replace", axum::routing::post(models::replace_model))
        .route("/models/:id", axum::routing::delete(models::delete_model))
        .nest("/workspace", workspace::router())
}

async fn health() -> &'static str {
    "ok"
}
