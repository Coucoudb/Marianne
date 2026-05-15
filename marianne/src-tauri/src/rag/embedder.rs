// src-tauri/src/rag/embedder.rs
use anyhow::Result;
use std::path::Path;

/// Dimension des embeddings multilingual-e5-small
pub const EMBEDDING_DIMS: usize = 384;

/// Initialiser le modèle d'embeddings (stub — fastembed désactivé pour CUDA)
pub fn init_embedder(_models_dir: &Path) -> Result<()> {
    // TODO Phase 3 : réactiver fastembed quand le conflit CRT sera résolu
    tracing::debug!("Embedder stub — fastembed désactivé");
    Ok(())
}

/// Embed une seule question (pour la recherche)
pub fn embed_query(_text: &str) -> Result<Vec<f32>> {
    anyhow::bail!("Embedder non disponible (fastembed désactivé)")
}

/// Embed un batch de passages (pour l'ingestion)
pub fn embed_passages(_texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    anyhow::bail!("Embedder non disponible (fastembed désactivé)")
}
