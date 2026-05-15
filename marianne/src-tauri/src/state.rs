// src-tauri/src/state.rs
use crate::llm::engine::LlmEngine;
use crate::rag::store::VectorStore;
use crate::rag::graph::KnowledgeGraph;
use crate::history::sqlite::HistoryDb;
use parking_lot::Mutex;
use std::sync::Arc;

/// État global partagé entre toutes les commandes Tauri
pub struct AppState {
    /// Moteur LLM (Candle) — protégé par Mutex car non thread-safe
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
}

impl AppState {
    pub fn new(data_dir: std::path::PathBuf) -> Self {
        Self {
            llm: Arc::new(Mutex::new(None)),
            vector_store: Arc::new(VectorStore::new(&data_dir.join("db"))),
            knowledge_graph: Arc::new(Mutex::new(KnowledgeGraph::new())),
            history: Arc::new(HistoryDb::new(&data_dir.join("history.db"))),
            last_llm_use: Arc::new(Mutex::new(None)),
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
}
