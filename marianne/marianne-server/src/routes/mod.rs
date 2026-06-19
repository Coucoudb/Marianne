// marianne-server/src/routes/mod.rs

pub mod admin;
pub mod chat;
pub mod documents;
pub mod history;
pub mod models;
pub mod profile;
pub mod system;
pub mod workspace;

use crate::middleware;
use crate::state::ServerState;
use axum::{middleware as axum_middleware, routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn build_router(state: ServerState) -> Router {
    // Routes utilisateur (auth requise, tout rôle)
    let user_routes = Router::new()
        .route("/chat", axum::routing::post(chat::chat_handler))
        .route(
            "/history/conversations",
            get(history::list_conversations_handler),
        )
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
        .nest("/workspace", workspace::router())
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::auth_middleware,
        ));

    // Routes admin (auth + rôle admin requis)
    let admin_routes = Router::new()
        .route("/admin/keys", axum::routing::post(admin::create_key))
        .route("/admin/keys", get(admin::list_keys))
        .route(
            "/admin/keys/:key_hash",
            axum::routing::delete(admin::revoke_key),
        )
        .route(
            "/models/download",
            axum::routing::post(models::download_model),
        )
        .route("/models/load", axum::routing::post(models::load_model))
        .route("/models/setup", axum::routing::post(models::setup_model))
        .route(
            "/models/replace",
            axum::routing::post(models::replace_model),
        )
        .route(
            "/models/:id",
            axum::routing::delete(models::delete_model),
        )
        .route_layer(axum_middleware::from_fn(
            middleware::auth::require_admin,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::auth_middleware,
        ));

    // Routes publiques
    let public_routes = Router::new()
        .route("/system/info", get(system::get_system_info))
        .route("/models/status", get(models::get_models_status));

    Router::new()
        .route("/health", get(health))
        .nest("/api/v1", user_routes.merge(admin_routes).merge(public_routes))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
