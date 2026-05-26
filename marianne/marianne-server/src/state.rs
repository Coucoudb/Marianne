// marianne-server/src/state.rs
// État partagé du serveur HTTP — wrapper autour du AppState du core.

use marianne_core::state::AppState;
use std::sync::Arc;

/// État injecté dans les handlers Axum via `axum::extract::State`.
#[derive(Clone)]
pub struct ServerState {
    pub core: Arc<AppState>,
}

impl ServerState {
    pub fn new(core: AppState) -> Self {
        Self {
            core: Arc::new(core),
        }
    }
}
