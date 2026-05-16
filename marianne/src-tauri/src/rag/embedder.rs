// src-tauri/src/rag/embedder.rs
use anyhow::Result;
use std::path::Path;

/// Dimension des embeddings multilingual-e5-small
pub const EMBEDDING_DIMS: usize = 384;

// ═══════════════════════════════════════════════════════════════════
// Backend fastembed (ORT) — toujours utilisé (indépendant du backend LLM)
// ═══════════════════════════════════════════════════════════════════
#[cfg(feature = "fastembed")]
mod backend {
    use super::*;
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;

    static EMBEDDER: OnceCell<Mutex<TextEmbedding>> = OnceCell::new();

    pub fn init_embedder(models_dir: &Path) -> Result<()> {
        if EMBEDDER.get().is_some() {
            return Ok(());
        }

        let cache_dir = models_dir.join("embeddings");
        std::fs::create_dir_all(&cache_dir)?;

        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::MultilingualE5Small;
        options.cache_dir = cache_dir.into();
        options.show_download_progress = true;
        let model = TextEmbedding::try_new(options)?;

        EMBEDDER
            .set(Mutex::new(model))
            .map_err(|_| anyhow::anyhow!("Embedder déjà initialisé"))?;

        tracing::info!("✅ Embedder initialisé (fastembed, multilingual-e5-small, {} dims)", EMBEDDING_DIMS);
        Ok(())
    }

    pub fn embed_query(text: &str) -> Result<Vec<f32>> {
        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;
        let guard = embedder
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock empoisonné: {}", e))?;

        let prefixed = format!("query: {}", text);
        let embeddings = guard.embed(vec![prefixed], None)?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Aucun embedding produit"))
    }

    pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;
        let guard = embedder
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock empoisonné: {}", e))?;

        let prefixed: Vec<String> = texts.iter().map(|t| format!("passage: {}", t)).collect();
        let embeddings = guard.embed(prefixed, None)?;
        Ok(embeddings)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Fallback si fastembed n'est pas disponible — embeddings désactivés
// ═══════════════════════════════════════════════════════════════════
#[cfg(not(feature = "fastembed"))]
mod backend {
    use super::*;

    pub fn init_embedder(_models_dir: &Path) -> Result<()> {
        tracing::warn!("Embeddings désactivés (fastembed non disponible)");
        Ok(())
    }

    pub fn embed_query(_text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.0; EMBEDDING_DIMS])
    }

    pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0; EMBEDDING_DIMS]).collect())
    }
}

// ═══════════════════════════════════════════════════════════════════
// API publique — délègue au backend actif
// ═══════════════════════════════════════════════════════════════════

pub fn init_embedder(models_dir: &Path) -> Result<()> {
    backend::init_embedder(models_dir)
}

pub fn embed_query(text: &str) -> Result<Vec<f32>> {
    backend::embed_query(text)
}

pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    backend::embed_passages(texts)
}
