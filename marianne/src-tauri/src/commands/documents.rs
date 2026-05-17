use crate::documents::extractor::{DocumentExtractor, DocumentType};
use tauri::State;
use crate::state::AppState;

#[derive(serde::Deserialize)]
pub struct AnalyzeDocumentRequest {
    pub file_path: String,
    pub question: Option<String>,
}

#[derive(serde::Serialize)]
pub struct ExtractedDocument {
    pub text: String,
    pub file_name: String,
    pub char_count: usize,
    pub prompt: String,
}

/// Extraire le texte d'un document et construire le prompt d'analyse
/// Le frontend envoie ensuite ce prompt via send_message pour bénéficier du streaming
#[tauri::command]
pub async fn extract_document(
    _state: State<'_, AppState>,
    request: AnalyzeDocumentRequest,
) -> Result<ExtractedDocument, String> {
    let path = std::path::Path::new(&request.file_path);

    if !path.exists() {
        return Err("Fichier introuvable.".to_string());
    }

    // Security: canonicalize path and reject directory traversal
    let canonical = path.canonicalize().map_err(|_| "Chemin de fichier invalide.".to_string())?;

    // Block access to system directories
    let path_str = canonical.to_string_lossy().to_lowercase();
    let blocked_prefixes = ["c:\\windows", "c:\\program", "/etc", "/usr", "/bin", "/sbin", "/var"];
    if blocked_prefixes.iter().any(|p| path_str.starts_with(p)) {
        return Err("Accès refusé : répertoire système protégé.".to_string());
    }

    let path = &canonical;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();

    // Extraire le texte selon le type de fichier
    let text = match DocumentExtractor::detect_type(path) {
        DocumentType::Pdf => {
            // Extraction PDF dans un thread bloquant (peut être lourd)
            let path_owned = path.to_path_buf();
            tokio::task::spawn_blocking(move || DocumentExtractor::extract_pdf(&path_owned))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?
        }
        DocumentType::Text => {
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| e.to_string())?
        }
        DocumentType::Image(_) => {
            return Err(
                "L'analyse d'images nécessite un modèle multimodal. Utilisez copier-coller pour l'instant."
                    .to_string(),
            );
        }
        DocumentType::Unknown => {
            return Err("Format de fichier non supporté. Formats acceptés : PDF, TXT, MD.".to_string());
        }
    };

    // Limiter à 4000 caractères pour rester dans la fenêtre de contexte
    let truncated: String = text.chars().take(4000).collect();
    let char_count = truncated.len();

    let question = request
        .question
        .unwrap_or_else(|| "Explique ce document en langage clair et dis-moi ce que je dois faire.".to_string());

    let prompt = format!(
        "Voici un document administratif français ({}) :\n\n\
         ---\n{}\n---\n\n\
         Question : {}",
        file_name, truncated, question
    );

    Ok(ExtractedDocument {
        text: truncated,
        file_name,
        char_count,
        prompt,
    })
}
