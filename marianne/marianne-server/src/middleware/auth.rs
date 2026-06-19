// marianne-server/src/middleware/auth.rs
// Middleware d'authentification par clé API (Bearer token).
//
// Security:
// - Clé hashée en SHA-256 AVANT le lookup DB — jamais stockée brute (OWASP A02).
// - Le hash est calculé dans ApiKeyDb::lookup ; aucun secret ne transite hors du core.

use crate::state::ServerState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use marianne_core::auth::api_keys::Role;
use serde_json::json;

/// Identifiant utilisateur injecté dans les extensions de la requête.
/// Les handlers en aval l'extraient via `Extension<UserId>`.
#[derive(Clone, Debug)]
pub struct UserId(pub String);

/// Rôle de l'utilisateur injecté dans les extensions de la requête.
#[derive(Clone, Debug)]
pub struct UserRole(pub Role);

pub async fn auth_middleware(
    State(state): State<ServerState>,
    mut request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let raw_key = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => return unauthorized(),
    };

    match state.core.api_keys.lookup(raw_key).await {
        Ok(Some(record)) => {
            request.extensions_mut().insert(UserId(record.user_id));
            request.extensions_mut().insert(UserRole(record.role));
            next.run(request).await
        }
        Ok(None) => unauthorized(),
        Err(e) => {
            tracing::error!("auth DB error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal error"})),
            )
                .into_response()
        }
    }
}

/// Middleware guard : exige UserRole == Admin.
///
/// # Axum layer ordering
/// In `routes/mod.rs`, add the `route_layer` for this middleware BEFORE the one
/// for `auth_middleware`. Axum applies the LAST `route_layer` call first, so
/// `auth_middleware` will run first (authenticating the request and inserting
/// `UserRole`), then this guard checks the role.
pub async fn require_admin(request: Request, next: Next) -> Response {
    let role = request.extensions().get::<UserRole>().cloned();
    match role {
        Some(UserRole(Role::Admin)) => next.run(request).await,
        _ => (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin required"})),
        )
            .into_response(),
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Unauthorized"})),
    )
        .into_response()
}
