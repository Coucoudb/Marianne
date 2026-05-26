// src-tauri/src/rag/ingestion.rs
use super::{
    embedder::{embed_passages, init_embedder},
    store::{KnowledgeChunk, VectorStore},
};
use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

/// Découper un texte en chunks de taille optimale.
/// Respecte les frontières de paragraphes et de phrases.
pub fn semantic_chunk(text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
    let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for paragraph in paragraphs {
        if paragraph.len() > max_chars {
            let sentences: Vec<&str> = paragraph
                .split(|c| c == '.' || c == '!' || c == '?')
                .filter(|s| !s.trim().is_empty())
                .collect();

            for sentence in sentences {
                if current_chunk.len() + sentence.len() > max_chars && !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    let overlap_text: String = current_chunk
                        .chars()
                        .rev()
                        .take(overlap)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    current_chunk = overlap_text;
                }
                current_chunk.push_str(sentence);
                current_chunk.push_str(". ");
            }
        } else {
            if current_chunk.len() + paragraph.len() > max_chars && !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
            }
            current_chunk.push_str(paragraph);
            current_chunk.push_str("\n\n");
        }
    }

    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

/// Ingérer tous les fichiers Markdown du corpus
pub async fn ingest_corpus(
    corpus_dir: &Path,
    store: &VectorStore,
    models_dir: &Path,
) -> Result<usize> {
    init_embedder(models_dir)?;
    store.ensure_table().await?;

    let mut total_chunks = 0;

    let mut entries = tokio::fs::read_dir(corpus_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let filename = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = tokio::fs::read_to_string(&path).await?;
        let raw_chunks = semantic_chunk(&content, 800, 100);
        let chunk_texts: Vec<&str> = raw_chunks.iter().map(|s| s.as_str()).collect();

        tracing::info!("Ingestion de {} : {} chunks", filename, raw_chunks.len());

        for batch in chunk_texts.chunks(32) {
            let embeddings = embed_passages(batch)?;

            let knowledge_chunks: Vec<KnowledgeChunk> = batch
                .iter()
                .zip(embeddings.iter())
                .map(|(text, embedding)| KnowledgeChunk {
                    id: Uuid::new_v4().to_string(),
                    text: text.to_string(),
                    source: filename.clone(),
                    category: categorize_source(&filename),
                    embedding: embedding.clone(),
                })
                .collect();

            total_chunks += store.insert_chunks(&knowledge_chunks).await?;
        }
    }

    tracing::info!("✅ Corpus ingéré : {} chunks total", total_chunks);
    Ok(total_chunks)
}

/// Déduire la catégorie depuis le nom de fichier
fn categorize_source(filename: &str) -> String {
    match filename {
        n if n.contains("travail") => "droit_travail",
        n if n.contains("caf") || n.contains("famille") => "caf",
        n if n.contains("urssaf") || n.contains("autoentrepreneur") => "urssaf",
        n if n.contains("logement") || n.contains("locataire") => "logement",
        n if n.contains("retraite") => "retraite",
        n if n.contains("recours") || n.contains("contestation") => "recours",
        _ => "general",
    }
    .to_string()
}
