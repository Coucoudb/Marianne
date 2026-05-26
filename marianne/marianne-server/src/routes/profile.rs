// marianne-server/src/routes/profile.rs
use crate::state::ServerState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use marianne_core::profile::UserProfile;

pub async fn get_profile_handler(
    State(server): State<ServerState>,
) -> Json<UserProfile> {
    Json(server.core.profile.lock().clone())
}

pub async fn update_profile_handler(
    State(server): State<ServerState>,
    Json(profile): Json<UserProfile>,
) -> StatusCode {
    if let Err(e) = profile.save(&server.core.data_dir) {
        tracing::warn!("profile save error: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    *server.core.profile.lock() = profile;
    StatusCode::NO_CONTENT
}
