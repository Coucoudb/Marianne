// src-tauri/src/rag/graph.rs
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Concept(String),
    Article { code: String, numero: String },
    Organisme(String),
    Formulaire(String),
    Statut(String),
    Droit(String),
    Condition(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeType {
    GerePar,
    NecessiteDe,
    ReferenceDans,
    PeutToucher,
    RempliAvec,
    LieA,
    ModifieCondition,
}

pub struct KnowledgeGraph {
    pub graph: DiGraph<NodeType, EdgeType>,
    pub index: HashMap<String, NodeIndex>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, name: &str, node_type: NodeType) -> NodeIndex {
        if let Some(&existing) = self.index.get(name) {
            return existing;
        }
        let idx = self.graph.add_node(node_type);
        self.index.insert(name.to_string(), idx);
        idx
    }

    pub fn add_edge(&mut self, from: &str, to: &str, edge_type: EdgeType) {
        if let (Some(&a), Some(&b)) = (self.index.get(from), self.index.get(to)) {
            self.graph.add_edge(a, b, edge_type);
        }
    }

    /// Multi-Hop BFS : récupère tous les nœuds à max_hops sauts
    pub fn expand_neighbors(&self, seed_name: &str, max_hops: usize) -> Vec<String> {
        let Some(&start_idx) = self.index.get(seed_name) else {
            return Vec::new();
        };

        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current_level = vec![start_idx];

        for _hop in 0..max_hops {
            let mut next_level = Vec::new();
            for &node_idx in &current_level {
                for neighbor in self.graph.neighbors(node_idx) {
                    if visited.insert(neighbor) {
                        if let Some(node) = self.graph.node_weight(neighbor) {
                            result.push(format!("{:?}", node));
                        }
                        next_level.push(neighbor);
                    }
                }
            }
            current_level = next_level;
            if current_level.is_empty() {
                break;
            }
        }

        result
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let data = bincode::serialize(&(&self.graph, &self.index))?;
        std::fs::write(path, data)?;
        tracing::info!("✅ Graphe sauvegardé : {} nœuds", self.graph.node_count());
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let data = std::fs::read(path)?;
        let (graph, index) = bincode::deserialize(&data)?;
        Ok(Self { graph, index })
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}
