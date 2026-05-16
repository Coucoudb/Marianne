// src-tauri/src/corpus/updater.rs
//
// Mise à jour hebdomadaire automatique du corpus légal français.
// Télécharge les fiches officielles, compare le hash, et réingère si changé.

use crate::rag::ingestion::semantic_chunk;
use crate::rag::store::{KnowledgeChunk, VectorStore};
use crate::web::rag_updater::content_hash;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Sources officielles à maintenir à jour automatiquement.
/// Format : (nom_unique, url, catégorie)
const UPDATE_SOURCES: &[(&str, &str, &str)] = &[
    (
        "prime_activite_conditions",
        "https://www.service-public.fr/particuliers/vosdroits/F2882",
        "caf",
    ),
    (
        "smic_taux",
        "https://www.service-public.fr/particuliers/vosdroits/F2889",
        "droit_travail",
    ),
    (
        "cotisations_ae",
        "https://www.autoentrepreneur.urssaf.fr/portail/accueil/sinformer-sur-le-statut/lessentiel-du-statut.html",
        "urssaf",
    ),
    (
        "apl_calcul",
        "https://www.service-public.fr/particuliers/vosdroits/F12006",
        "caf",
    ),
    (
        "preavis_licenciement",
        "https://www.service-public.fr/particuliers/vosdroits/F31392",
        "droit_travail",
    ),
    (
        "indemnite_licenciement",
        "https://www.service-public.fr/particuliers/vosdroits/F987",
        "droit_travail",
    ),
    (
        "rsa_conditions",
        "https://www.service-public.fr/particuliers/vosdroits/F19778",
        "caf",
    ),
    (
        "droits_locataire",
        "https://www.service-public.fr/particuliers/vosdroits/F31269",
        "logement",
    ),
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateReport {
    pub updated: usize,
    pub unchanged: usize,
    pub failed: usize,
}

pub struct CorpusUpdater {
    client: reqwest::Client,
    store: Arc<VectorStore>,
    hashes_dir: PathBuf,
}

impl CorpusUpdater {
    pub fn new(store: Arc<VectorStore>, data_dir: &Path) -> Self {
        let hashes_dir = data_dir.join("corpus_hashes");
        std::fs::create_dir_all(&hashes_dir).ok();

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (compatible; MarianneBot/1.0)")
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .unwrap_or_default();

        Self {
            client,
            store,
            hashes_dir,
        }
    }

    /// Exécuter la mise à jour de toutes les sources
    pub async fn run_update(&self) -> Result<UpdateReport> {
        let mut updated = 0usize;
        let mut unchanged = 0usize;
        let mut failed = 0usize;

        for (name, url, category) in UPDATE_SOURCES {
            match self.update_one_source(name, url, category).await {
                Ok(true) => updated += 1,
                Ok(false) => unchanged += 1,
                Err(e) => {
                    tracing::warn!("Mise à jour corpus '{}' échouée : {}", name, e);
                    failed += 1;
                }
            }
        }

        tracing::info!(
            "Mise à jour corpus terminée : {} mis à jour, {} inchangés, {} échoués",
            updated,
            unchanged,
            failed
        );

        Ok(UpdateReport {
            updated,
            unchanged,
            failed,
        })
    }

    /// Mettre à jour une source individuelle. Retourne Ok(true) si mis à jour, Ok(false) si inchangé.
    async fn update_one_source(
        &self,
        name: &str,
        url: &str,
        category: &str,
    ) -> Result<bool> {
        // 1. Télécharger la page
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {}", response.status());
        }
        let html = response.text().await?;
        let text = extract_main_text(&html);

        if text.len() < 100 {
            anyhow::bail!("Contenu trop court ({} chars)", text.len());
        }

        // 2. Comparer le hash
        let new_hash = content_hash(&text);
        let stored_hash = self.load_hash(name);

        if Some(&new_hash) == stored_hash.as_ref() {
            return Ok(false); // Inchangé
        }

        // 3. Supprimer les anciens chunks de cette source
        let source_tag = format!("corpus:{}", name);
        self.store.delete_by_source(&source_tag).await?;

        // 4. Découper et ingérer
        let chunks = semantic_chunk(&text, 800, 100);

        #[cfg(feature = "fastembed")]
        {
            let texts: Vec<&str> = chunks.iter().map(|s| s.as_str()).collect();
            if let Ok(embeddings) = crate::rag::embedder::embed_passages(&texts) {
                let knowledge_chunks: Vec<KnowledgeChunk> = chunks
                    .iter()
                    .zip(embeddings.iter())
                    .map(|(text, emb)| KnowledgeChunk {
                        id: uuid::Uuid::new_v4().to_string(),
                        text: text.clone(),
                        source: source_tag.clone(),
                        category: category.to_string(),
                        embedding: emb.clone(),
                    })
                    .collect();

                self.store.insert_chunks(&knowledge_chunks).await?;
            }
        }

        // 5. Sauvegarder le hash
        self.save_hash(name, &new_hash);
        tracing::info!("✅ Corpus mis à jour : {} ({} chunks)", name, chunks.len());
        Ok(true)
    }

    fn load_hash(&self, name: &str) -> Option<String> {
        let path = self.hashes_dir.join(format!("{}.hash", name));
        std::fs::read_to_string(path).ok()
    }

    fn save_hash(&self, name: &str, hash: &str) {
        let path = self.hashes_dir.join(format!("{}.hash", name));
        std::fs::write(path, hash).ok();
    }
}

/// Extraire le texte principal d'une page HTML (contenu éditorial)
fn extract_main_text(html: &str) -> String {
    use scraper::{Html, Selector};

    let document = Html::parse_document(html);

    // Sélecteurs courants pour les sites gouvernementaux français
    let content_selectors = [
        "article",
        ".article-body",
        ".text-content",
        "main .content",
        "#main-content",
        ".sp-article-body",
        ".field--name-body",
        "main",
    ];

    for selector_str in &content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            let texts: Vec<String> = document
                .select(&selector)
                .flat_map(|el| {
                    el.text()
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                })
                .collect();

            if texts.len() > 3 {
                return texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
            }
        }
    }

    // Fallback : tout le texte du body
    document
        .root_element()
        .text()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .take(500)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lire le timestamp de la dernière mise à jour
pub fn read_last_update_timestamp(data_dir: &Path) -> Option<std::time::SystemTime> {
    let path = data_dir.join("corpus_hashes").join("_last_update");
    std::fs::metadata(&path).ok().and_then(|m| m.modified().ok())
}

/// Sauvegarder le timestamp de mise à jour
pub fn save_last_update_timestamp(data_dir: &Path) {
    let path = data_dir.join("corpus_hashes").join("_last_update");
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    std::fs::write(&path, "").ok();
}

/// Vérifier si une mise à jour est nécessaire (> 7 jours)
pub fn needs_update(data_dir: &Path) -> bool {
    let one_week = Duration::from_secs(7 * 24 * 3600);
    match read_last_update_timestamp(data_dir) {
        Some(last) => {
            last.elapsed().map(|e| e > one_week).unwrap_or(true)
        }
        None => true, // Jamais mis à jour
    }
}
