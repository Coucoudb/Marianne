// src-tauri/src/rag/retriever.rs
use super::{
    embedder::embed_query,
    store::{SearchResult, VectorStore},
};
use anyhow::Result;
use std::sync::Arc;

pub struct Retriever {
    store: Arc<VectorStore>,
}

impl Retriever {
    pub fn new(store: Arc<VectorStore>) -> Self {
        Self { store }
    }

    /// Rechercher les passages les plus pertinents pour une question
    pub async fn retrieve(&self, question: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        let query_embedding = embed_query(question)?;

        let results = self.store.search(query_embedding, top_k, None).await?;

        tracing::debug!(
            "RAG : {} résultats pour '{}'",
            results.len(),
            &question[..50.min(question.len())]
        );

        Ok(results)
    }

    /// Formater les résultats en contexte pour le LLM
    pub fn format_context(results: &[SearchResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut context = String::from("CONTEXTE LÉGAL ET RÉGLEMENTAIRE :\n\n");

        for (i, result) in results.iter().enumerate() {
            context.push_str(&format!(
                "Passage {} (source : {}) :\n{}\n\n",
                i + 1,
                result.source,
                result.text
            ));
        }

        context
    }
}
