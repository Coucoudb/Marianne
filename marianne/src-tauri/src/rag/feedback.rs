// src-tauri/src/rag/feedback.rs
//! Feedback loop : injecte les résultats web réussis dans le RAG
//! pour que Marianne apprenne au fil du temps.

use super::{
    embedder::embed_passages,
    ingestion::semantic_chunk,
    store::{KnowledgeChunk, VectorStore},
};
use crate::web::searcher::WebResult;
use dashmap::DashSet;
use std::sync::Arc;
use uuid::Uuid;

/// Ingère des résultats web dans le vector store avec déduplication par hash.
///
/// - Chaque passage web est chunké (800 chars, 100 overlap) comme le corpus
/// - Source préfixée "web:{source_name}" pour distinguer des docs locaux
/// - Déduplication via xxhash sur le contenu textuel brut
///
/// Retourne le nombre de nouveaux chunks insérés.
pub async fn ingest_web_results(
    results: &[WebResult],
    store: &VectorStore,
    known_hashes: &Arc<DashSet<String>>,
    category: &str,
) -> anyhow::Result<usize> {
    if results.is_empty() {
        return Ok(0);
    }

    let mut all_chunks: Vec<KnowledgeChunk> = Vec::new();
    let mut texts_to_embed: Vec<String> = Vec::new();
    let mut chunk_meta: Vec<(String, String)> = Vec::new(); // (source, category)

    for result in results {
        if result.content.trim().len() < 200 {
            continue; // Contenu trop court ou de mauvaise qualité, pas utile
        }

        let source = format!("web:{}", result.source_name);
        let chunks = semantic_chunk(&result.content, 800, 100);

        for chunk_text in chunks {
            // Déduplication par xxhash
            let hash = xxhash_rust::xxh3::xxh3_64(chunk_text.as_bytes());
            let hash_str = format!("{:x}", hash);

            if known_hashes.contains(&hash_str) {
                tracing::debug!("Chunk web déjà connu (hash={}), skip", &hash_str[..8]);
                continue;
            }

            known_hashes.insert(hash_str);
            texts_to_embed.push(chunk_text);
            chunk_meta.push((source.clone(), category.to_string()));
        }
    }

    if texts_to_embed.is_empty() {
        tracing::debug!("Aucun nouveau chunk web à insérer (tous dédupliqués)");
        return Ok(0);
    }

    // Embedding par batches de 32 (même logique que ingest_corpus)
    for batch_start in (0..texts_to_embed.len()).step_by(32) {
        let batch_end = (batch_start + 32).min(texts_to_embed.len());
        let batch_refs: Vec<&str> = texts_to_embed[batch_start..batch_end]
            .iter()
            .map(|s| s.as_str())
            .collect();

        let embeddings = embed_passages(&batch_refs)?;

        let knowledge_chunks: Vec<KnowledgeChunk> = batch_refs
            .iter()
            .zip(embeddings.iter())
            .enumerate()
            .map(|(i, (text, embedding))| {
                let idx = batch_start + i;
                KnowledgeChunk {
                    id: Uuid::new_v4().to_string(),
                    text: text.to_string(),
                    source: chunk_meta[idx].0.clone(),
                    category: chunk_meta[idx].1.clone(),
                    embedding: embedding.clone(),
                }
            })
            .collect();

        all_chunks.extend(knowledge_chunks);
    }

    let count = store.insert_chunks(&all_chunks).await?;
    tracing::info!(
        "🧠 Feedback loop : {} nouveaux chunks web injectés dans le RAG",
        count
    );

    Ok(count)
}
