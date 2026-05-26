// marianne-core/src/models.rs
// Gestion du registre des modèles GGUF installés localement.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Entrée dans le registre local des modèles installés
#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledModel {
    /// Identifiant unique (slug du repo + filename)
    pub id: String,
    /// Repo HuggingFace (ex: "microsoft/Phi-3-mini-4k-instruct-gguf")
    pub repo_id: String,
    /// Nom du fichier GGUF sur disque
    pub filename: String,
    /// Nom lisible du modèle
    pub name: String,
    /// Taille en Mo
    pub size_mb: u64,
}

/// Charger le registre des modèles installés depuis data_dir/models/registry.json
pub fn load_installed_models(data_dir: &Path) -> Vec<InstalledModel> {
    let path = data_dir.join("models").join("registry.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Sauvegarder le registre des modèles installés
pub fn save_installed_models(data_dir: &Path, models: &[InstalledModel]) -> anyhow::Result<()> {
    let dir = data_dir.join("models");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("registry.json");
    std::fs::write(&path, serde_json::to_string_pretty(models)?)?;
    Ok(())
}

/// Résoudre le nom de fichier GGUF à partir de l'identifiant du modèle sélectionné.
/// Cherche d'abord dans le registre local, puis utilise un fallback de noms connus.
pub fn resolve_model_filename(data_dir: &Path, selected_model: &str) -> String {
    let installed = load_installed_models(data_dir);
    installed
        .iter()
        .find(|m| m.id == selected_model)
        .map(|m| m.filename.clone())
        .unwrap_or_else(|| {
            match selected_model {
                "phi-3-mini-q4" => "Phi-3-mini-4k-instruct-q4.gguf".to_string(),
                "phi-3.5-mini-q4" => "Phi-3.5-mini-instruct-Q4_K_M.gguf".to_string(),
                "phi-3-medium-q4" => "Phi-3-medium-4k-instruct-Q4_K_M.gguf".to_string(),
                _ => format!("{}.gguf", selected_model),
            }
        })
}
