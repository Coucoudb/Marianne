// marianne-server/src/routes/documents.rs
use crate::state::ServerState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use marianne_core::documents::extractor::{DocumentExtractor, DocumentType};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ExtractRequest {
    pub file_path: String,
    pub question: Option<String>,
}

#[derive(Serialize)]
pub struct ExtractedDocument {
    pub text: String,
    pub file_name: String,
    pub char_count: usize,
    pub prompt: String,
}

pub async fn extract_handler(
    State(_server): State<ServerState>,
    Json(request): Json<ExtractRequest>,
) -> Result<Json<ExtractedDocument>, (StatusCode, String)> {
    let path = std::path::Path::new(&request.file_path);

    if !path.exists() {
        return Err((StatusCode::NOT_FOUND, "Fichier introuvable.".into()));
    }

    let canonical = path.canonicalize().map_err(|_| {
        (StatusCode::BAD_REQUEST, "Chemin de fichier invalide.".into())
    })?;

    let path_str = canonical.to_string_lossy().to_lowercase();
    let blocked = ["c:\\windows", "c:\\program", "/etc", "/usr", "/bin", "/sbin", "/var"];
    if blocked.iter().any(|p| path_str.starts_with(p)) {
        return Err((StatusCode::FORBIDDEN, "Accès refusé : répertoire système protégé.".into()));
    }

    let file_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    let path_owned = canonical.clone();
    let text = tokio::task::spawn_blocking(move || match DocumentExtractor::detect_type(&path_owned) {
        DocumentType::Pdf => DocumentExtractor::extract_pdf(&path_owned),
        DocumentType::Text => std::fs::read_to_string(&path_owned).map_err(Into::into),
        DocumentType::Image(_) => Err(anyhow::anyhow!(
            "L'analyse d'images nécessite un modèle multimodal. Utilisez copier-coller pour l'instant."
        )),
        DocumentType::Unknown => Err(anyhow::anyhow!("Format de fichier non supporté.")),
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;

    let question = request.question.as_deref().unwrap_or("Résume ce document.");
    let prompt = format!(
        "Document : {}\n\nContenu :\n{}\n\nQuestion : {}\nRéponse :",
        file_name,
        &text[..text.len().min(8000)],
        question
    );

    Ok(Json(ExtractedDocument {
        char_count: text.len(),
        text,
        file_name,
        prompt,
    }))
}
