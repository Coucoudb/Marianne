// src-tauri/src/rag/ingestion.rs
use super::{
    embedder::{embed_passages, init_embedder},
    store::{KnowledgeChunk, VectorStore},
};
use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

/// Découper un texte en chunks de taille optimale avec heuristiques sémantiques.
/// Respecte les frontières de paragraphes, de titres, et maintient le contexte.
pub fn semantic_chunk(text: &str, max_chars: usize) -> Vec<String> {
    let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_section_title = String::new();

    for paragraph in paragraphs {
        let trimmed_para = paragraph.trim();
        
        // Détecter un titre de section
        if trimmed_para.starts_with('#') {
            if let Some(title) = trimmed_para.lines().next() {
                current_section_title = title.to_string();
            }
            
            // Forcer une coupure de chunk avant une nouvelle section si le chunk actuel est assez grand
            if current_chunk.len() > 200 {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
            }
        }

        // Si le paragraphe est lui-même plus grand que le max (rare mais possible)
        if trimmed_para.len() > max_chars {
            let sentences: Vec<&str> = trimmed_para
                .split(['.', '!', '?'])
                .filter(|s| !s.trim().is_empty())
                .collect();

            for sentence in sentences {
                if current_chunk.len() + sentence.len() > max_chars && !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    
                    // Overlap: garder la dernière phrase comme contexte
                    let last_sentence = current_chunk.split(". ").last().unwrap_or("").to_string();
                    current_chunk = format!("{} [Suite de {}] ... {}", last_sentence, current_section_title, sentence);
                } else {
                    current_chunk.push_str(sentence);
                    current_chunk.push_str(". ");
                }
            }
        } else {
            if current_chunk.len() + trimmed_para.len() > max_chars && !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                
                // Overlap: garder le dernier paragraphe du chunk précédent
                let last_para = current_chunk.split("\n\n").last().unwrap_or("").to_string();
                
                // Commencer le nouveau chunk avec le titre de section et l'overlap
                current_chunk = String::new();
                if !current_section_title.is_empty() {
                    current_chunk.push_str(&format!("Contexte : {}\n\n", current_section_title));
                }
                current_chunk.push_str(&last_para);
                current_chunk.push_str("\n\n");
            }
            
            // Si on commence un nouveau chunk, inclure le titre
            if current_chunk.is_empty() && !current_section_title.is_empty() && !trimmed_para.starts_with('#') {
                current_chunk.push_str(&format!("Contexte : {}\n\n", current_section_title));
            }
            
            current_chunk.push_str(trimmed_para);
            current_chunk.push_str("\n\n");
        }
    }

    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    // Fusionner les micro-chunks (< 200 chars) avec le précédent s'il y en a un
    let mut final_chunks: Vec<String> = Vec::new();
    for chunk in chunks {
        if chunk.len() < 200 && !final_chunks.is_empty() {
            let mut last = final_chunks.pop().unwrap();
            last.push_str("\n\n");
            last.push_str(&chunk);
            final_chunks.push(last);
        } else {
            final_chunks.push(chunk);
        }
    }

    final_chunks
}

/// Extraire dynamiquement des tags depuis le nom de fichier et le début du texte
fn dynamic_categorize(filename: &str, text_sample: &str) -> String {
    let mut tags = std::collections::HashSet::new();
    let text_lower = text_sample.to_lowercase();
    let file_lower = filename.to_lowercase();
    
    let keywords = [
        ("caf", vec!["caf", "allocation", "apl", "rsa", "prime d'activité"]),
        ("urssaf", vec!["urssaf", "autoentrepreneur", "micro-entreprise", "cotisations"]),
        ("sante", vec!["ameli", "sécurité sociale", "cpam", "maladie", "mutuelle"]),
        ("droit_travail", vec!["travail", "contrat", "licenciement", "démission", "cdd", "cdi"]),
        ("logement", vec!["logement", "locataire", "propriétaire", "bail", "loyer"]),
        ("retraite", vec!["retraite", "pension", "trimestres"]),
        ("recours", vec!["recours", "contestation", "litige", "tribunal"]),
        ("impots", vec!["impôts", "fiscal", "déclaration", "revenus"]),
    ];

    // Vérifier le nom de fichier
    for (tag, aliases) in &keywords {
        if aliases.iter().any(|a| file_lower.contains(a)) {
            tags.insert(tag.to_string());
        }
    }

    // Analyser le contenu (premiers caractères)
    for (tag, aliases) in &keywords {
        let hits = aliases.iter().filter(|a| text_lower.contains(*a)).count();
        if hits > 0 {
            tags.insert(tag.to_string());
        }
    }

    if tags.is_empty() {
        tags.insert("general".to_string());
    }

    let tags_vec: Vec<String> = tags.into_iter().collect();
    serde_json::to_string(&tags_vec).unwrap_or_else(|_| "[\"general\"]".to_string())
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
        let raw_chunks = semantic_chunk(&content, 800);
        let chunk_texts: Vec<&str> = raw_chunks.iter().map(|s| s.as_str()).collect();

        tracing::info!("Ingestion de {} : {} chunks", filename, raw_chunks.len());

        for batch in chunk_texts.chunks(32) {
            let embeddings = embed_passages(batch)?;

            let knowledge_chunks: Vec<KnowledgeChunk> = batch
                .iter()
                .zip(embeddings.iter())
                .map(|(text, embedding)| {
                    // Prendre un échantillon du texte pour la catégorisation dynamique
                    let sample: String = text.chars().take(1000).collect();
                    KnowledgeChunk {
                        id: Uuid::new_v4().to_string(),
                        text: text.to_string(),
                        source: filename.clone(),
                        tags: dynamic_categorize(&filename, &sample),
                        embedding: embedding.clone(),
                    }
                })
                .collect();

            total_chunks += store.insert_chunks(&knowledge_chunks).await?;
        }
    }

    tracing::info!("✅ Corpus ingéré : {} chunks total", total_chunks);
    Ok(total_chunks)
}
