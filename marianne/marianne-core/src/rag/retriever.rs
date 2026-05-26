// src-tauri/src/rag/retriever.rs
//! Pipeline RAG amélioré :
//! 1. Expansion de requête via Knowledge Graph
//! 2. Recherche hybride (sémantique + BM25 FTS)
//! 3. Fusion RRF (Reciprocal Rank Fusion)
//! 4. Re-ranking (fraîcheur, autorité, catégorie)

use super::{
    embedder::embed_query,
    graph::KnowledgeGraph,
    store::{SearchResult, VectorStore},
};
use anyhow::Result;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

/// Facteurs de re-ranking
const AUTHORITY_BOOST: f32 = 0.15;
const FRESHNESS_BOOST: f32 = 0.10;
const CATEGORY_BOOST: f32 = 0.08;
const RRF_K: f32 = 60.0; // constante RRF standard

pub struct Retriever {
    store: Arc<VectorStore>,
    graph: Arc<Mutex<KnowledgeGraph>>,
}

/// Résultat enrichi après re-ranking
#[derive(Debug, Clone)]
pub struct RankedResult {
    pub text: String,
    pub source: String,
    pub score: f32,
    pub is_web_source: bool,
}

impl Retriever {
    pub fn new(store: Arc<VectorStore>, graph: Arc<Mutex<KnowledgeGraph>>) -> Self {
        Self { store, graph }
    }

    /// Pipeline complet : expansion → recherche hybride → RRF → re-ranking
    pub async fn retrieve(
        &self,
        question: &str,
        top_k: usize,
        category: Option<&str>,
    ) -> Result<Vec<RankedResult>> {
        // 1. Expansion de requête via le Knowledge Graph
        let expanded_terms = self.expand_query(question);

        // 2. Recherche sémantique (vectorielle)
        let query_embedding = embed_query(question)?;
        let semantic_results = self.store.search(query_embedding, top_k * 2, None).await?;

        // 3. Recherche FTS (BM25) — termes exacts + expansion
        let fts_query = self.build_fts_query(question, &expanded_terms);
        let fts_results = self.store.search_fts(&fts_query, top_k * 2).await
            .unwrap_or_default();

        // 4. Fusion RRF des deux listes
        let fused = self.rrf_fusion(&semantic_results, &fts_results);

        // 5. Re-ranking (autorité, fraîcheur, catégorie)
        let ranked = self.rerank(fused, category);

        // 6. Tronquer au top_k final
        let final_results: Vec<RankedResult> = ranked.into_iter().take(top_k).collect();

        tracing::debug!(
            "RAG pipeline : {} résultats (semantic={}, fts={}, expanded_terms={})",
            final_results.len(),
            semantic_results.len(),
            fts_results.len(),
            expanded_terms.len(),
        );

        Ok(final_results)
    }

    /// Expansion de requête via le Knowledge Graph
    /// Trouve les concepts voisins pour enrichir la recherche
    fn expand_query(&self, question: &str) -> Vec<String> {
        let graph = self.graph.lock();
        if graph.node_count() == 0 {
            return Vec::new();
        }

        let q_lower = question.to_lowercase();
        let mut expanded = Vec::new();

        // Chercher les nœuds du graphe mentionnés dans la question
        for (name, _) in graph.index.iter() {
            let name_lower = name.to_lowercase();
            if q_lower.contains(&name_lower) {
                // Expansion multi-hop (1 saut) pour les voisins directs
                let neighbors = graph.expand_neighbors(name, 1);
                for neighbor in neighbors.into_iter().take(5) {
                    // Extraire le texte utile du NodeType debug format
                    let clean = extract_node_label(&neighbor);
                    if !clean.is_empty() && !expanded.contains(&clean) {
                        expanded.push(clean);
                    }
                }
            }
        }

        // Limiter à 8 termes d'expansion max
        expanded.truncate(8);
        if !expanded.is_empty() {
            tracing::debug!("Graph expansion : {:?}", expanded);
        }
        expanded
    }

    /// Construire la requête FTS en combinant la question et les termes d'expansion
    fn build_fts_query(&self, question: &str, expanded_terms: &[String]) -> String {
        let stop_words = [
            "quel", "quelle", "quels", "quelles", "est", "sont", "dans", "pour",
            "avec", "cette", "les", "des", "une", "que", "qui", "comment",
            "plus", "moins", "par", "sur", "aux", "fait", "faire", "peut",
            "entre", "comme", "mais", "aussi", "tout", "tous", "bien",
            "avoir", "être", "quoi", "quand", "pourquoi", "combien",
            "mon", "mes", "votre", "notre", "leur", "dois", "doit", "faut",
        ];

        let mut keywords: Vec<String> = question
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
            .filter(|w| w.len() > 3 && !stop_words.contains(w))
            .map(|w| w.to_string())
            .collect();

        // Ajouter les termes d'expansion du graphe
        for term in expanded_terms {
            let t = term.to_lowercase();
            if !keywords.contains(&t) {
                keywords.push(t);
            }
        }

        keywords.join(" ")
    }

    /// Reciprocal Rank Fusion : combine les résultats de 2 systèmes de recherche
    fn rrf_fusion(
        &self,
        semantic: &[SearchResult],
        fts: &[SearchResult],
    ) -> Vec<SearchResult> {
        let mut scores: HashMap<String, (f32, SearchResult)> = HashMap::new();

        // Scores sémantiques
        for (rank, result) in semantic.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
            let key = result.text[..result.text.len().min(100)].to_string();
            scores
                .entry(key)
                .and_modify(|(s, _)| *s += rrf_score)
                .or_insert((rrf_score, result.clone()));
        }

        // Scores FTS
        for (rank, result) in fts.iter().enumerate() {
            let rrf_score = 1.0 / (RRF_K + rank as f32 + 1.0);
            let key = result.text[..result.text.len().min(100)].to_string();
            scores
                .entry(key)
                .and_modify(|(s, _)| *s += rrf_score)
                .or_insert((rrf_score, result.clone()));
        }

        // Trier par score RRF décroissant
        let mut fused: Vec<(f32, SearchResult)> = scores.into_values().collect();
        fused.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        fused.into_iter().map(|(_, r)| r).collect()
    }

    /// Re-ranking multi-facteurs
    fn rerank(&self, results: Vec<SearchResult>, category: Option<&str>) -> Vec<RankedResult> {
        let mut ranked: Vec<RankedResult> = results
            .into_iter()
            .map(|r| {
                let mut score = r.similarity;
                let is_web = r.source.starts_with("web:");

                // Boost d'autorité : sources officielles > web > inconnues
                score += authority_score(&r.source) * AUTHORITY_BOOST;

                // Boost de fraîcheur : sources web récentes > corpus statique
                if is_web {
                    score += FRESHNESS_BOOST;
                }

                // Boost de catégorie : si la source correspond à la catégorie détectée
                if let Some(cat) = category {
                    if source_matches_category(&r.source, cat) {
                        score += CATEGORY_BOOST;
                    }
                }

                RankedResult {
                    text: r.text,
                    source: r.source,
                    score,
                    is_web_source: is_web,
                }
            })
            .collect();

        ranked.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Formater les résultats en contexte pour le LLM
    pub fn format_context(results: &[RankedResult]) -> String {
        if results.is_empty() {
            return String::new();
        }

        let mut context = String::from("CONTEXTE LÉGAL ET RÉGLEMENTAIRE :\n\n");

        for (i, result) in results.iter().enumerate() {
            let reliability = if result.source.contains("legifrance") {
                "source officielle — Légifrance"
            } else if result.source.contains("service-public") || result.source.contains(".gouv") {
                "source officielle"
            } else if result.is_web_source {
                "source web"
            } else {
                "corpus local"
            };

            context.push_str(&format!(
                "Passage {} ({}, {}) :\n{}\n\n",
                i + 1,
                result.source.trim_start_matches("web:"),
                reliability,
                result.text
            ));
        }

        context
    }

    /// Détecter les contradictions entre sources web et corpus local
    pub fn detect_contradictions(results: &[RankedResult]) -> Option<String> {
        let web_results: Vec<&RankedResult> = results.iter().filter(|r| r.is_web_source).collect();
        let local_results: Vec<&RankedResult> = results.iter().filter(|r| !r.is_web_source).collect();

        if web_results.is_empty() || local_results.is_empty() {
            return None;
        }

        // Détecter des contradictions potentielles via des marqueurs
        let contradiction_markers = [
            ("ne plus", "peut"), ("supprimé", "existe"), ("abrogé", "en vigueur"),
            ("ancien", "nouveau"), ("remplacé", "applicable"), ("modifié", "inchangé"),
            ("depuis le", "jusqu'au"),
        ];

        for web in &web_results {
            let web_lower = web.text.to_lowercase();
            for local in &local_results {
                let local_lower = local.text.to_lowercase();
                for (marker_new, marker_old) in &contradiction_markers {
                    if web_lower.contains(marker_new) && local_lower.contains(marker_old) {
                        return Some(format!(
                            "⚠️ Des informations récentes (source: {}) pourraient modifier les données de la base locale. Privilégiez la source la plus récente.",
                            web.source.trim_start_matches("web:")
                        ));
                    }
                }
            }
        }

        None
    }
}

/// Score d'autorité d'une source (0.0 - 1.0)
fn authority_score(source: &str) -> f32 {
    let s = source.to_lowercase();
    if s.contains("legifrance") {
        1.0
    } else if s.contains("service-public") || s.contains(".gouv") {
        0.9
    } else if s.contains("ameli") || s.contains("caf") || s.contains("urssaf") {
        0.85
    } else if s.contains("france-travail") || s.contains("francetravail") {
        0.8
    } else if s.starts_with("web:") {
        0.5
    } else {
        0.7 // corpus local
    }
}

/// Vérifier si une source correspond à la catégorie détectée
fn source_matches_category(source: &str, category: &str) -> bool {
    let s = source.to_lowercase();
    match category {
        "caf" => s.contains("caf") || s.contains("apl") || s.contains("rsa") || s.contains("prime"),
        "sante" => s.contains("ameli") || s.contains("securite_sociale") || s.contains("sante"),
        "urssaf" => s.contains("urssaf") || s.contains("autoentrepreneur"),
        "chomage" => s.contains("chomage") || s.contains("are") || s.contains("france-travail"),
        "impots" => s.contains("impots") || s.contains("fiscal"),
        "identite" => s.contains("identite") || s.contains("document"),
        "droit_travail" => s.contains("travail") || s.contains("contrat") || s.contains("licenciement"),
        "logement" => s.contains("logement") || s.contains("locataire"),
        "retraite" => s.contains("retraite") || s.contains("droits"),
        "recours" => s.contains("recours") || s.contains("contestation"),
        _ => false,
    }
}

/// Extraire un label lisible d'un NodeType au format Debug
fn extract_node_label(debug_str: &str) -> String {
    if let Some(start) = debug_str.find('"') {
        if let Some(end) = debug_str[start + 1..].find('"') {
            return debug_str[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}
