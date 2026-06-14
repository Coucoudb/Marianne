use crate::state::ServerState;
use axum::{
    extract::{Path, State, Query},
    routing::get,
    Json, Router,
};
use marianne_core::workspace::agent::Agent;
use marianne_core::workspace::skill::Skill;
use marianne_core::workspace::manager::SaveLevel;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct SaveQuery {
    #[serde(default)]
    level: SaveLevel,
}

#[derive(Deserialize)]
struct ProjectDirRequest {
    path: Option<String>,
}

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/agents", get(list_agents).post(create_agent))
        .route("/agents/:id", get(get_agent).put(update_agent).delete(delete_agent))
        .route("/skills", get(list_skills).post(create_skill))
        .route("/skills/:id", get(get_skill).put(update_skill).delete(delete_skill))
        .route("/project-dir", axum::routing::post(set_project_dir).get(get_project_dir))
}

// ─── Project Dir ────────────────────────────────────────────────────────────

async fn set_project_dir(
    State(state): State<ServerState>,
    Json(payload): Json<ProjectDirRequest>,
) -> Json<serde_json::Value> {
    match payload.path {
        Some(p) => {
            let path = std::path::PathBuf::from(&p);
            if path.exists() {
                state.core.workspace.set_project_dir(Some(path));
                Json(json!({ "status": "success", "project_dir": p }))
            } else {
                Json(json!({ "status": "error", "message": format!("Le chemin '{}' n'existe pas", p) }))
            }
        }
        None => {
            state.core.workspace.set_project_dir(None);
            Json(json!({ "status": "success", "project_dir": null }))
        }
    }
}

async fn get_project_dir(
    State(state): State<ServerState>,
) -> Json<serde_json::Value> {
    let dir = state.core.workspace.get_project_dir();
    Json(json!({ "status": "success", "project_dir": dir.map(|d| d.to_string_lossy().to_string()) }))
}

// ─── Agents ─────────────────────────────────────────────────────────────────

async fn list_agents(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.core.workspace.list_agents().await {
        Ok(agents) => {
            let enriched: Vec<serde_json::Value> = agents.iter().map(|a| {
                let level = state.core.workspace.get_agent_level(&a.id);
                let mut val = serde_json::to_value(a).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("level".to_string(), serde_json::to_value(&level).unwrap_or_default());
                }
                val
            }).collect();
            Json(json!({ "status": "success", "data": enriched }))
        }
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
                let level = state.core.workspace.get_agent_level(&agent.id);
                let mut val = serde_json::to_value(&agent).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("level".to_string(), serde_json::to_value(&level).unwrap_or_default());
                }
                Json(json!({ "status": "success", "data": val }))
            } else {
                Json(json!({ "status": "error", "message": "Agent non trouvé" }))
            }
        }
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn create_agent(
    State(state): State<ServerState>,
    Query(query): Query<SaveQuery>,
    Json(payload): Json<Agent>,
) -> Json<serde_json::Value> {
    match state.core.workspace.save_agent(&payload, query.level).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn update_agent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Query(query): Query<SaveQuery>,
    Json(mut payload): Json<Agent>,
) -> Json<serde_json::Value> {
    payload.id = id;
    match state.core.workspace.save_agent(&payload, query.level).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn delete_agent(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if state.core.workspace.delete_agent(&id).await.is_ok() {
        Json(json!({ "status": "success" }))
    } else {
        Json(json!({ "status": "error", "message": "Erreur lors de la suppression" }))
    }
}

// ─── Skills ─────────────────────────────────────────────────────────────────

async fn list_skills(State(state): State<ServerState>) -> Json<serde_json::Value> {
    match state.core.workspace.list_skills().await {
        Ok(skills) => {
            let enriched: Vec<serde_json::Value> = skills.iter().map(|s| {
                let level = state.core.workspace.get_skill_level(&s.id);
                let mut val = serde_json::to_value(s).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("level".to_string(), serde_json::to_value(&level).unwrap_or_default());
                }
                val
            }).collect();
            Json(json!({ "status": "success", "data": enriched }))
        }
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
                let level = state.core.workspace.get_skill_level(&skill.id);
                let mut val = serde_json::to_value(&skill).unwrap_or_default();
                if let Some(obj) = val.as_object_mut() {
                    obj.insert("level".to_string(), serde_json::to_value(&level).unwrap_or_default());
                }
                Json(json!({ "status": "success", "data": val }))
            } else {
                Json(json!({ "status": "error", "message": "Skill non trouvé" }))
            }
        }
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn create_skill(
    State(state): State<ServerState>,
    Query(query): Query<SaveQuery>,
    Json(payload): Json<Skill>,
) -> Json<serde_json::Value> {
    match state.core.workspace.save_skill(&payload, query.level).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn update_skill(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Query(query): Query<SaveQuery>,
    Json(mut payload): Json<Skill>,
) -> Json<serde_json::Value> {
    payload.id = id;
    match state.core.workspace.save_skill(&payload, query.level).await {
        Ok(_) => Json(json!({ "status": "success", "data": payload })),
        Err(e) => Json(json!({ "status": "error", "message": e.to_string() })),
    }
}

async fn delete_skill(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    if state.core.workspace.delete_skill(&id).await.is_ok() {
        Json(json!({ "status": "success" }))
    } else {
        Json(json!({ "status": "error", "message": "Erreur lors de la suppression" }))
    }
}
