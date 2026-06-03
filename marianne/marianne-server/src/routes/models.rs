// marianne-server/src/routes/models.rs
// Endpoints pour la gestion des modèles LLM (download, load, status).

use crate::state::ServerState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use marianne_core::models::{load_installed_models, InstalledModel};
use marianne_core::setup::download_model_from_huggingface;
use serde::{Deserialize, Serialize};

// ─── Types de requête/réponse ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DownloadRequest {
    pub repo_id: String,
    pub filename: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct DownloadResponse {
    pub status: String,
    pub model_id: String,
}

#[derive(Deserialize)]
pub struct LoadRequest {
    pub model_id: String,
}

#[derive(Serialize)]
pub struct LoadResponse {
    pub status: String,
    pub model_name: String,
}

#[derive(Serialize)]
pub struct ModelStatusResponse {
    pub downloaded_models: Vec<ModelEntry>,
    pub loaded_model: Option<LoadedModelInfo>,
}

#[derive(Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub filename: String,
    pub size_mb: u64,
    pub repo_id: String,
}

#[derive(Serialize)]
pub struct LoadedModelInfo {
    pub id: String,
    pub name: String,
    pub device: String,
}

#[derive(Serialize)]
pub struct SetupResponse {
    pub status: String,
    pub message: String,
}

// ─── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/v1/models/status — Retourne le statut des modèles
pub async fn get_models_status(State(server): State<ServerState>) -> Json<ModelStatusResponse> {
    let installed = load_installed_models(&server.core.data_dir);
    let models_dir = server.core.data_dir.join("models");

    let downloaded_models: Vec<ModelEntry> = installed
        .into_iter()
        .filter(|m| models_dir.join(&m.filename).exists())
        .map(|m| ModelEntry {
            id: m.id,
            name: m.name,
            filename: m.filename,
            size_mb: m.size_mb,
            repo_id: m.repo_id,
        })
        .collect();

    let loaded_model = if server.core.is_model_loaded() {
        let profile = server.core.profile.lock().clone();
        let device_preference = profile.device_preference;
        let device_str = match device_preference {
            marianne_core::profile::DevicePreference::Gpu => "gpu",
            marianne_core::profile::DevicePreference::Cpu => "cpu",
        };

        // Trouver le modèle actif dans la liste des modèles installés
        let model = load_installed_models(&server.core.data_dir)
            .into_iter()
            .find(|m| m.id == profile.selected_model);

        model.map(|m| LoadedModelInfo {
            id: m.id,
            name: m.name,
            device: device_str.to_string(),
        })
    } else {
        None
    };

    Json(ModelStatusResponse {
        downloaded_models,
        loaded_model,
    })
}

/// POST /api/v1/models/download — Télécharge un modèle depuis HuggingFace
/// Retourne immédiatement un ID de modèle, le téléchargement se fait en arrière-plan.
pub async fn download_model(
    State(server): State<ServerState>,
    Json(req): Json<DownloadRequest>,
) -> Result<Json<DownloadResponse>, AppError> {
    let data_dir = server.core.data_dir.clone();
    let repo_id = req.repo_id.clone();
    let filename = req.filename.clone();
    let name = req.name.clone();

    // Lancer le téléchargement en arrière-plan
    tokio::spawn(async move {
        match download_model_from_huggingface(&data_dir, &repo_id, &filename, &name, |_| {}).await {
            Ok(model_id) => {
                tracing::info!("✅ Modèle {} téléchargé avec succès", model_id);
            }
            Err(e) => {
                tracing::error!("❌ Échec du téléchargement : {}", e);
            }
        }
    });

    // Générer l'ID immédiatement pour la réponse
    let model_id = format!(
        "{}_{}",
        req.repo_id.replace('/', "_"),
        req.filename.trim_end_matches(".gguf")
    );

    Ok(Json(DownloadResponse {
        status: "downloading".to_string(),
        model_id,
    }))
}

/// POST /api/v1/models/load — Charge un modèle téléchargé en mémoire
pub async fn load_model(
    State(server): State<ServerState>,
    Json(req): Json<LoadRequest>,
) -> Result<Json<LoadResponse>, AppError> {
    let installed = load_installed_models(&server.core.data_dir);
    let model = installed
        .iter()
        .find(|m| m.id == req.model_id)
        .ok_or_else(|| AppError::NotFound("Modèle introuvable".to_string()))?;

    let models_dir = server.core.data_dir.join("models");
    if !models_dir.join(&model.filename).exists() {
        return Err(AppError::NotFound("Fichier du modèle introuvable".to_string()));
    }

    // Mettre à jour le profil pour sélectionner ce modèle
    {
        let mut profile = server.core.profile.lock();
        profile.selected_model = req.model_id.clone();
        profile.save(&server.core.data_dir).map_err(|e| {
            AppError::Internal(format!("Impossible de sauvegarder le profil : {}", e))
        })?;
    }

    // Charger le modèle en mémoire
    marianne_core::setup::load_model_into_memory(&server.core)
        .await
        .map_err(|e| AppError::Internal(format!("Échec du chargement : {}", e)))?;

    Ok(Json(LoadResponse {
        status: "loaded".to_string(),
        model_name: model.name.clone(),
    }))
}

/// POST /api/v1/models/setup — Réexécute l'initialisation complète (download + load + RAG)
pub async fn setup_model(State(server): State<ServerState>) -> Result<Json<SetupResponse>, AppError> {
    marianne_core::setup::ensure_model_ready(&server.core)
        .await
        .map_err(|e| AppError::Internal(format!("Échec de l'initialisation : {}", e)))?;

    Ok(Json(SetupResponse {
        status: "ready".to_string(),
        message: "Marianne est prête".to_string(),
    }))
}

// ─── Gestion d'erreur ────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(serde_json::json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
