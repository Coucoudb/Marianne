use crate::state::ServerState;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use marianne_core::workspace::agent::Agent;
use marianne_core::workspace::skill::Skill;
use serde_json::json;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/:id", get(get_agent).put(update_agent).delete(delete_agent))
        .route("/skills", get(list_skills).post(create_skill))
        .route("/skills/:id", get(get_skill).put(update_skill).delete(delete_skill))
}

// ─── Agents ─────────────────────────────────────────────────────────────────

async fn list_agents(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.core.workspace.list_agents().await {
        Ok(agents) => Json(json!({ "status": "success", "data": agents })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn get_agent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.core.workspace.list_agents().await {
        Ok(agents) => {
            if let Some(agent) = agents.into_iter().find(|a| a.id == id) {
                Json(json!({ "status": "success", "data": agent }))
            } else {
                Json(json!({ "status": "error", "message": "Agent non trouvé" }))
            }
        }
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn create_agent(
    State(state): State<ServerState>,
    Json(payload): Json<Agent>,
) -> Json<serde_json::Value> {
    match state.core.workspace.save_agent(&payload).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn update_agent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(mut payload): Json<Agent>,
) -> Json<serde_json::Value> {
    payload.id = id;
    match state.core.workspace.save_agent(&payload).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn delete_agent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = state.core.data_dir.join("workspace").join("agents").join(format!("{}.json", id));
    if tokio::fs::remove_file(path).await.is_ok() {
        Json(json!({ "status": "success" }))
    } else {
        Json(json!({ "status": "error", "message": "Erreur lors de la suppression" }))
    }
}

// ─── Skills ─────────────────────────────────────────────────────────────────

async fn list_skills(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.core.workspace.list_skills().await {
        Ok(skills) => Json(json!({ "status": "success", "data": skills })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn get_skill(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    match state.core.workspace.list_skills().await {
        Ok(skills) => {
            if let Some(skill) = skills.into_iter().find(|s| s.id == id) {
                Json(json!({ "status": "success", "data": skill }))
            } else {
                Json(json!({ "status": "error", "message": "Skill non trouvé" }))
            }
        }
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn create_skill(
    State(state): State<ServerState>,
    Json(payload): Json<Skill>,
) -> Json<serde_json::Value> {
    match state.core.workspace.save_skill(&payload).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn update_skill(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(mut payload): Json<Skill>,
) -> Json<serde_json::Value> {
    payload.id = id;
    match state.core.workspace.save_skill(&payload).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn delete_skill(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let path = state.core.data_dir.join("workspace").join("skills").join(format!("{}.json", id));
    if tokio::fs::remove_file(path).await.is_ok() {
        Json(json!({ "status": "success" }))
    } else {
        Json(json!({ "status": "error", "message": "Erreur lors de la suppression" }))
    }
}
