// src-tauri/src/rag/store.rs
use anyhow::Result;
use std::path::Path;

/// Dimensions des embeddings
use super::embedder::EMBEDDING_DIMS;

pub struct VectorStore {
    db_path: std::path::PathBuf,
}

impl VectorStore {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    /// Créer la table si elle n'existe pas
    pub async fn ensure_table(&self) -> Result<()> {
        // TODO Phase 3 : connecter à LanceDB et créer la table
        tracing::debug!("VectorStore::ensure_table — stub (LanceDB non activé)");
        Ok(())
    }

    /// Insérer des chunks avec leurs embeddings
    pub async fn insert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<usize> {
        if chunks.is_empty() {
            return Ok(0);
        }
        // TODO Phase 3 : implémenter l'insertion Arrow dans LanceDB
        tracing::info!("✅ {} chunks à insérer (stub)", chunks.len());
        Ok(chunks.len())
    }

    /// Recherche sémantique : trouve les k chunks les plus pertinents
    pub async fn search(
        &self,
        _query_embedding: Vec<f32>,
        _top_k: usize,
        _category_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        // TODO Phase 3 : implémenter la recherche vectorielle LanceDB
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgeChunk {
    pub id: String,
    pub text: String,
    pub source: String,
    pub category: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub text: String,
    pub source: String,
    pub score: f32,
}
