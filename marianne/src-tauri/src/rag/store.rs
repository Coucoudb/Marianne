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
        let conn = self.connect().await?;
        let tables = conn.table_names().execute().await?;

        if !tables.contains(&"knowledge".to_string()) {
            tracing::info!("Création de la table 'knowledge' dans LanceDB");
            // TODO Phase 3 : créer la table avec le schéma Arrow complet
        }

        Ok(())
    }

    /// Insérer des chunks avec leurs embeddings
    pub async fn insert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<usize> {
        if chunks.is_empty() {
            return Ok(0);
        }
        // TODO Phase 3 : implémenter l'insertion Arrow dans LanceDB
        tracing::info!("✅ {} chunks à insérer dans LanceDB", chunks.len());
        Ok(chunks.len())
    }

    /// Recherche sémantique : trouve les k chunks les plus pertinents
    pub async fn search(
        &self,
        query_embedding: Vec<f32>,
        top_k: usize,
        _category_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        // TODO Phase 3 : implémenter la recherche vectorielle LanceDB
        Ok(Vec::new())
    }

    async fn connect(&self) -> Result<lancedb::Connection> {
        let uri = self
            .db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Chemin invalide"))?;
        Ok(lancedb::connect(uri).execute().await?)
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub text: String,
    pub source: String,
    pub similarity: f32,
}
