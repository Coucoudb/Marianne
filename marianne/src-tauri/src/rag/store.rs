// src-tauri/src/rag/store.rs
use anyhow::Result;
use std::path::Path;

/// Dimensions des embeddings
use super::embedder::EMBEDDING_DIMS;

// ═══════════════════════════════════════════════════════════════════
// Structs communes (indépendantes du backend)
// ═══════════════════════════════════════════════════════════════════

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

// ═══════════════════════════════════════════════════════════════════
// Backend LanceDB (feature "vectordb")
// ═══════════════════════════════════════════════════════════════════
#[cfg(feature = "vectordb")]
mod backend {
    use super::*;
    use lancedb::{Connection, query::{QueryBase, ExecutableQuery}};
    use arrow_array::{
        Array, RecordBatch, RecordBatchIterator, RecordBatchReader,
        StringArray, FixedSizeListArray, Float32Array, ArrayRef,
    };
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    use futures_util::TryStreamExt;

    struct KnowledgeSchema;

    impl KnowledgeSchema {
        fn arrow_schema() -> Arc<Schema> {
            Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("text", DataType::Utf8, false),
                Field::new("source", DataType::Utf8, false),
                Field::new("category", DataType::Utf8, false),
                Field::new(
                    "embedding",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        EMBEDDING_DIMS as i32,
                    ),
                    false,
                ),
            ]))
        }
    }

    pub struct VectorStore {
        db_path: std::path::PathBuf,
    }

    impl VectorStore {
        pub fn new(db_path: &Path) -> Self {
            Self { db_path: db_path.to_path_buf() }
        }

        async fn connect(&self) -> Result<Connection> {
            let uri = self.db_path.to_str()
                .ok_or_else(|| anyhow::anyhow!("Chemin invalide"))?;
            Ok(lancedb::connect(uri).execute().await?)
        }

        pub async fn ensure_table(&self) -> Result<()> {
            let conn = self.connect().await?;
            let tables = conn.table_names().execute().await?;

            if !tables.contains(&"knowledge".to_string()) {
                let schema = KnowledgeSchema::arrow_schema();
                conn.create_empty_table("knowledge", schema).execute().await?;
                tracing::info!("✅ Table 'knowledge' créée dans LanceDB");
            }
            Ok(())
        }

        /// Construire un RecordBatch à partir de chunks
        fn chunks_to_batch(chunks: &[KnowledgeChunk]) -> Result<RecordBatch> {
            let schema = KnowledgeSchema::arrow_schema();
            let ids: Vec<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
            let texts: Vec<&str> = chunks.iter().map(|c| c.text.as_str()).collect();
            let sources: Vec<&str> = chunks.iter().map(|c| c.source.as_str()).collect();
            let categories: Vec<&str> = chunks.iter().map(|c| c.category.as_str()).collect();

            let flat_embeddings: Vec<f32> = chunks.iter()
                .flat_map(|c| c.embedding.iter().cloned())
                .collect();

            let values: ArrayRef = Arc::new(Float32Array::from(flat_embeddings));
            let field = Arc::new(Field::new("item", DataType::Float32, true));
            let embedding_array = FixedSizeListArray::new(
                field, EMBEDDING_DIMS as i32, values, None,
            );

            Ok(RecordBatch::try_new(schema, vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(StringArray::from(texts)),
                Arc::new(StringArray::from(sources)),
                Arc::new(StringArray::from(categories)),
                Arc::new(embedding_array),
            ])?)
        }

        pub async fn insert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<usize> {
            if chunks.is_empty() { return Ok(0); }

            let conn = self.connect().await?;
            let tables = conn.table_names().execute().await?;
            let batch = Self::chunks_to_batch(chunks)?;
            let schema = batch.schema();

            if !tables.contains(&"knowledge".to_string()) {
                // Créer la table directement avec les données (évite le bug empty table)
                let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
                let reader: Box<dyn RecordBatchReader + Send> = Box::new(batches);
                conn.create_table("knowledge", reader).execute().await?;
                tracing::info!("✅ Table 'knowledge' créée avec {} chunks", chunks.len());
            } else {
                // Ajouter à la table existante
                let table = conn.open_table("knowledge").execute().await?;
                let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
                let reader: Box<dyn RecordBatchReader + Send> = Box::new(batches);
                table.add(reader).execute().await?;
                tracing::info!("✅ {} chunks insérés dans LanceDB", chunks.len());
            }

            Ok(chunks.len())
        }

        pub async fn search(
            &self,
            query_embedding: Vec<f32>,
            top_k: usize,
            category_filter: Option<&str>,
        ) -> Result<Vec<SearchResult>> {
            let conn = self.connect().await?;
            let tables = conn.table_names().execute().await?;
            if !tables.contains(&"knowledge".to_string()) {
                return Ok(Vec::new());
            }
            let table = conn.open_table("knowledge").execute().await?;
            if table.count_rows(None).await? == 0 {
                return Ok(Vec::new());
            }

            let mut query = table
                .vector_search(query_embedding)?
                .limit(top_k)
                .distance_type(lancedb::DistanceType::Cosine);

            if let Some(cat) = category_filter {
                query = query.only_if(format!("category = '{}'", cat));
            }

            let results = query.execute().await?.try_collect::<Vec<_>>().await?;

            let mut search_results = Vec::new();
            for batch in results {
                let text_col = batch.column_by_name("text")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                let source_col = batch.column_by_name("source")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                let distance_col = batch.column_by_name("_distance")
                    .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

                if let (Some(texts), Some(sources)) = (text_col, source_col) {
                    for i in 0..batch.num_rows() {
                        let distance = distance_col.map(|d| d.value(i)).unwrap_or(1.0);
                        let similarity = 1.0 - distance;
                        if similarity > 0.4 {
                            search_results.push(SearchResult {
                                text: texts.value(i).to_string(),
                                source: sources.value(i).to_string(),
                                similarity,
                            });
                        }
                    }
                }
            }

            search_results.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
            Ok(search_results)
        }

        /// Créer les index IVF-PQ + FTS après ingestion complète du corpus
        /// À appeler une seule fois — les index sont ensuite persistés sur disque
        pub async fn build_all_indexes(&self) -> Result<()> {
            let conn = self.connect().await?;
            let table = conn.open_table("knowledge").execute().await?;

            // 1. Index vectoriel IVF-PQ : O(n) → O(sqrt(n)) en recherche
            table.create_index(
                &["embedding"],
                lancedb::index::Index::IvfPq(
                    lancedb::index::vector::IvfPqIndexBuilder::default()
                        .num_partitions(32)
                        .num_sub_vectors(16)
                ),
            ).execute().await?;

            // 2. Index FTS BM25 : pour les numéros d'articles et Cerfa exacts
            table.create_index(
                &["text"],
                lancedb::index::Index::FTS(Default::default()),
            ).execute().await?;

            tracing::info!("✅ Index IVF-PQ + FTS créés sur LanceDB");
            Ok(())
        }

        /// Supprimer tous les chunks d'une source donnée
        pub async fn delete_by_source(&self, source: &str) -> Result<()> {
            let conn = self.connect().await?;
            let tables = conn.table_names().execute().await?;
            if !tables.contains(&"knowledge".to_string()) {
                return Ok(());
            }
            let table = conn.open_table("knowledge").execute().await?;
            table.delete(&format!("source = '{}'", source)).await?;
            tracing::info!("Supprimé les chunks de source '{}'", source);
            Ok(())
        }

        /// Charger les hashes de tous les chunks web existants pour déduplication
        pub async fn load_all_content_hashes(&self) -> Result<Vec<String>> {
            let conn = self.connect().await?;
            let tables = conn.table_names().execute().await?;
            if !tables.contains(&"knowledge".to_string()) {
                return Ok(Vec::new());
            }
            let table = conn.open_table("knowledge").execute().await?;
            if table.count_rows(None).await? == 0 {
                return Ok(Vec::new());
            }
            let results = table
                .query()
                .only_if("source LIKE 'web:%'")
                .select(lancedb::query::Select::Columns(vec!["text".to_string()]))
                .execute()
                .await?
                .try_collect::<Vec<_>>()
                .await?;

            let hashes: Vec<String> = results.iter()
                .flat_map(|batch| {
                    batch.column_by_name("text")
                        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                        .map(|arr| (0..arr.len()).filter_map(|i| {
                            if arr.is_valid(i) {
                                let hash = xxhash_rust::xxh3::xxh3_64(arr.value(i).as_bytes());
                                Some(format!("{:x}", hash))
                            } else { None }
                        }).collect::<Vec<_>>())
                        .unwrap_or_default()
                })
                .collect();

            tracing::info!("Chargé {} hashes web depuis LanceDB", hashes.len());
            Ok(hashes)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Backend en mémoire (mode CUDA — pas de LanceDB)
// Brute-force cosine similarity, suffisant pour un corpus < 10k chunks
// ═══════════════════════════════════════════════════════════════════
#[cfg(not(feature = "vectordb"))]
mod backend {
    use super::*;
    use parking_lot::RwLock;

    pub struct VectorStore {
        chunks: RwLock<Vec<KnowledgeChunk>>,
        _db_path: std::path::PathBuf,
    }

    impl VectorStore {
        pub fn new(db_path: &Path) -> Self {
            Self {
                chunks: RwLock::new(Vec::new()),
                _db_path: db_path.to_path_buf(),
            }
        }

        pub async fn ensure_table(&self) -> Result<()> {
            tracing::info!("VectorStore en mémoire (mode CUDA, pas de LanceDB)");
            Ok(())
        }

        pub async fn insert_chunks(&self, chunks: &[KnowledgeChunk]) -> Result<usize> {
            if chunks.is_empty() { return Ok(0); }
            let mut store = self.chunks.write();
            let count = chunks.len();
            store.extend(chunks.iter().cloned());
            tracing::info!("✅ {} chunks insérés (mémoire, total={})", count, store.len());
            Ok(count)
        }

        pub async fn search(
            &self,
            query_embedding: Vec<f32>,
            top_k: usize,
            category_filter: Option<&str>,
        ) -> Result<Vec<SearchResult>> {
            let store = self.chunks.read();

            let mut scored: Vec<SearchResult> = store.iter()
                .filter(|c| {
                    category_filter.map_or(true, |cat| c.category == cat)
                })
                .map(|c| {
                    let sim = cosine_similarity(&query_embedding, &c.embedding);
                    SearchResult {
                        text: c.text.clone(),
                        source: c.source.clone(),
                        similarity: sim,
                    }
                })
                .filter(|r| r.similarity > 0.4)
                .collect();

            scored.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
            scored.truncate(top_k);
            Ok(scored)
        }

        /// No-op en mode mémoire (pas d'index persistant)
        pub async fn build_all_indexes(&self) -> Result<()> {
            tracing::debug!("build_all_indexes: no-op en mode mémoire");
            Ok(())
        }

        /// Supprimer tous les chunks d'une source donnée
        pub async fn delete_by_source(&self, source: &str) -> Result<()> {
            let mut store = self.chunks.write();
            let before = store.len();
            store.retain(|c| c.source != source);
            let removed = before - store.len();
            if removed > 0 {
                tracing::info!("Supprimé {} chunks de source '{}'", removed, source);
            }
            Ok(())
        }

        /// Charger les hashes de tous les chunks web existants pour déduplication
        pub async fn load_all_content_hashes(&self) -> Result<Vec<String>> {
            let store = self.chunks.read();
            let hashes: Vec<String> = store.iter()
                .filter(|c| c.source.starts_with("web:"))
                .map(|c| {
                    let hash = xxhash_rust::xxh3::xxh3_64(c.text.as_bytes());
                    format!("{:x}", hash)
                })
                .collect();
            Ok(hashes)
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() { return 0.0; }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
        dot / (norm_a * norm_b)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Re-export public
// ═══════════════════════════════════════════════════════════════════
pub use backend::VectorStore;
