// src-tauri/src/state.rs
use crate::llm::engine::LlmEngine;
use crate::network::connectivity::ConnectivityCache;
use crate::profile::UserProfile;
use crate::rag::store::VectorStore;
use crate::rag::graph::KnowledgeGraph;
use crate::history::sqlite::HistoryDb;
use dashmap::DashSet;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// État global partagé entre toutes les commandes Tauri
pub struct AppState {
    /// Moteur LLM (llama.cpp) — protégé par Mutex
    pub llm: Arc<Mutex<Option<LlmEngine>>>,

    /// Base vectorielle + FTS RAG — thread-safe en lecture
    pub vector_store: Arc<VectorStore>,

    /// Graphe de connaissances petgraph (GraphRAG)
    pub knowledge_graph: Arc<Mutex<KnowledgeGraph>>,

    /// Historique des conversations (SQLite)
    pub history: Arc<HistoryDb>,

    /// Chemin vers le répertoire de données de Marianne
    pub data_dir: std::path::PathBuf,

    /// Timestamp du dernier appel LLM (pour déchargement mémoire auto)
    pub last_llm_use: Arc<Mutex<Option<std::time::Instant>>>,

    /// HashSet de tous les content_hash connus — pour déduplication O(1)
    pub known_hashes: Arc<DashSet<String>>,

    /// Profil utilisateur (préférences persistantes)
    pub profile: Arc<Mutex<UserProfile>>,

    /// Cache de connectivité réseau (mode hors-ligne intelligent)
    pub connectivity: Arc<ConnectivityCache>,

    /// Flag d'arrêt de génération — positionné par stop_generation
    pub abort_generation: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        let profile = UserProfile::load(&data_dir);
        Self {
            llm: Arc::new(Mutex::new(None)),
            vector_store: Arc::new(VectorStore::new(&data_dir.join("db"))),
            knowledge_graph: Arc::new(Mutex::new(KnowledgeGraph::new())),
            history: Arc::new(HistoryDb::new(&data_dir.join("history.db"))),
            last_llm_use: Arc::new(Mutex::new(None)),
            known_hashes: Arc::new(DashSet::new()),
            profile: Arc::new(Mutex::new(profile)),
            connectivity: Arc::new(ConnectivityCache::new()),
            abort_generation: Arc::new(AtomicBool::new(false)),
            data_dir,
        }
    }

    pub fn is_model_loaded(&self) -> bool {
        self.llm.lock().is_some()
    }

    /// Mettre à jour le timestamp d'utilisation
    pub fn touch_llm(&self) {
        *self.last_llm_use.lock() = Some(std::time::Instant::now());
    }

    /// Libérer Phi-3-Mini de la RAM après 10 minutes d'inactivité
    /// Économie : ~3 Go libérés pour le reste du système
    pub fn maybe_unload_model(&self) {
        let should_unload = self.last_llm_use.lock()
            .map(|last| last.elapsed() > std::time::Duration::from_secs(600))
            .unwrap_or(false);

        if should_unload && self.is_model_loaded() {
            tracing::info!("Marianne inactive depuis 10min — libération RAM...");
            *self.llm.lock() = None;
        }
    }

    /// Recharger le modèle à la demande si nécessaire
    pub async fn ensure_model_loaded(&self) -> anyhow::Result<()> {
        if !self.is_model_loaded() {
            tracing::info!("Rechargement du modèle...");
            let dir = self.data_dir.join("models");
            let profile = self.profile.lock().clone();
            let device_preference = profile.device_preference.clone();
            let gpu_selection = profile.gpu_selection.clone();
            let selected_model = profile.selected_model.clone();

            // Résoudre le nom de fichier via le registre
            let model_filename = crate::commands::setup::resolve_model_filename(
                &self.data_dir,
                &selected_model,
            );

            let engine = tokio::task::spawn_blocking(move || {
                crate::llm::engine::LlmEngine::load(&dir, &device_preference, &gpu_selection, &model_filename)
            }).await??;
            *self.llm.lock() = Some(engine);
        }
        self.touch_llm();
        Ok(())
    }
}
