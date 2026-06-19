// marianne-server/src/routes/admin.rs
// Routes d'administration des clés API.
// Toutes protégées par require_admin (appliqué dans routes/mod.rs).

use crate::state::ServerState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use marianne_core::auth::api_keys::{ApiKeyRecord, Role};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateKeyRequest {
    pub user_id: String,
    pub label: Option<String>,
    pub role: Option<String>,
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub key: String,
    pub user_id: String,
    pub label: String,
    pub role: String,
}

pub async fn create_key(
    State(state): State<ServerState>,
    Json(body): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, StatusCode> {
    // Validate user_id: only alphanumeric, hyphen, underscore
    if body.user_id.is_empty()
        || !body
            .user_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let raw_key = format!("mk_{}", Uuid::new_v4().simple());
    let label = body.label.unwrap_or_default();
    let role = match body.role.as_deref() {
        Some("admin") => Role::Admin,
        _ => Role::User,
    };

    state
        .core
        .api_keys
        .insert(&raw_key, &body.user_id, &label, role.clone())
        .await
        .map_err(|e| {
            tracing::error!("create_key error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(CreateKeyResponse {
        key: raw_key,
        user_id: body.user_id,
        label,
        role: role.as_str().to_string(),
    }))
}

pub async fn list_keys(
    State(state): State<ServerState>,
) -> Result<Json<Vec<ApiKeyRecord>>, StatusCode> {
    state
        .core
        .api_keys
        .list_all()
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("list_keys error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

pub async fn revoke_key(
    State(state): State<ServerState>,
    Path(key_hash): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Validate key_hash is hex-only (64 hex chars = SHA-256)
    if key_hash.len() != 64 || !key_hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let found = state
        .core
        .api_keys
        .revoke(&key_hash)
        .await
        .map_err(|e| {
            tracing::error!("revoke error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if found {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
