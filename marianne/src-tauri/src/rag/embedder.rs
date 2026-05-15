// src-tauri/src/rag/embedder.rs
use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use once_cell::sync::OnceCell;
use std::path::Path;

/// Dimension des embeddings multilingual-e5-small
pub const EMBEDDING_DIMS: usize = 384;

static EMBEDDER: OnceCell<TextEmbedding> = OnceCell::new();

/// Initialiser le modèle d'embeddings (appelé une seule fois)
pub fn init_embedder(_models_dir: &Path) -> Result<()> {
    EMBEDDER.get_or_try_init(|| {
        TextEmbedding::try_new(InitOptions::new(EmbeddingModel::MultilingualE5Small))
            .map_err(|e| anyhow::anyhow!("Erreur init embedder : {}", e))
    })?;
    Ok(())
}

/// Embed une seule question (pour la recherche)
pub fn embed_query(text: &str) -> Result<Vec<f32>> {
    let embedder = EMBEDDER
        .get()
        .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;

    let embeddings = embedder
        .embed(vec![text], None)
        .map_err(|e| anyhow::anyhow!("Erreur embedding query : {}", e))?;

    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Embedding vide"))
}

/// Embed un batch de passages (pour l'ingestion)
pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let embedder = EMBEDDER
        .get()
        .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;

    let texts_owned: Vec<String> = texts.iter().map(|t| t.to_string()).collect();

    embedder
        .embed(texts_owned, None)
        .map_err(|e| anyhow::anyhow!("Erreur embedding passages : {}", e))
}
