// src-tauri/src/rag/embedder.rs
use anyhow::Result;
use std::path::Path;

/// Dimension des embeddings multilingual-e5-small
pub const EMBEDDING_DIMS: usize = 384;

// ═══════════════════════════════════════════════════════════════════
// Backend fastembed (ORT) — utilisé quand la feature cuda est absente
// ═══════════════════════════════════════════════════════════════════
#[cfg(feature = "fastembed")]
mod backend {
    use super::*;
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;

    static EMBEDDER: OnceCell<Mutex<TextEmbedding>> = OnceCell::new();

    pub fn init_embedder(models_dir: &Path) -> Result<()> {
        if EMBEDDER.get().is_some() {
            return Ok(());
        }

        let cache_dir = models_dir.join("embeddings");
        std::fs::create_dir_all(&cache_dir)?;

        let mut options = InitOptions::default();
        options.model_name = EmbeddingModel::MultilingualE5Small;
        options.cache_dir = cache_dir.into();
        options.show_download_progress = true;
        let model = TextEmbedding::try_new(options)?;

        EMBEDDER
            .set(Mutex::new(model))
            .map_err(|_| anyhow::anyhow!("Embedder déjà initialisé"))?;

        tracing::info!("✅ Embedder initialisé (fastembed, multilingual-e5-small, {} dims)", EMBEDDING_DIMS);
        Ok(())
    }

    pub fn embed_query(text: &str) -> Result<Vec<f32>> {
        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;
        let guard = embedder
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock empoisonné: {}", e))?;

        let prefixed = format!("query: {}", text);
        let embeddings = guard.embed(vec![prefixed], None)?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Aucun embedding produit"))
    }

    pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;
        let guard = embedder
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock empoisonné: {}", e))?;

        let prefixed: Vec<String> = texts.iter().map(|t| format!("passage: {}", t)).collect();
        let embeddings = guard.embed(prefixed, None)?;
        Ok(embeddings)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Backend Candle — utilisé en mode CUDA (pas de conflit CRT)
// ═══════════════════════════════════════════════════════════════════
#[cfg(not(feature = "fastembed"))]
mod backend {
    use super::*;
    use candle_core::{Device, Tensor};
    use candle_nn::VarBuilder;
    use once_cell::sync::OnceCell;
    use std::sync::Mutex;
    use tokenizers::Tokenizer;

    struct CandleEmbedder {
        model: candle_transformers::models::bert::BertModel,
        tokenizer: Tokenizer,
        device: Device,
    }

    static EMBEDDER: OnceCell<Mutex<CandleEmbedder>> = OnceCell::new();

    pub fn init_embedder(models_dir: &Path) -> Result<()> {
        if EMBEDDER.get().is_some() {
            return Ok(());
        }

        let model_dir = models_dir.join("multilingual-e5-small");
        if !model_dir.exists() {
            anyhow::bail!(
                "Modèle d'embeddings non trouvé dans {:?}. Téléchargez multilingual-e5-small.",
                model_dir
            );
        }

        let device = if cfg!(feature = "cuda") {
            Device::new_cuda(0)?
        } else {
            Device::Cpu
        };

        let tokenizer = Tokenizer::from_file(model_dir.join("tokenizer.json"))
            .map_err(|e| anyhow::anyhow!("Tokenizer: {}", e))?;

        let weights = candle_core::safetensors::load(
            model_dir.join("model.safetensors"),
            &device,
        )?;
        let vb = VarBuilder::from_tensors(weights, candle_core::DType::F32, &device);

        let config_str = std::fs::read_to_string(model_dir.join("config.json"))?;
        let config: candle_transformers::models::bert::Config =
            serde_json::from_str(&config_str)?;

        let model = candle_transformers::models::bert::BertModel::load(vb, &config)?;

        EMBEDDER
            .set(Mutex::new(CandleEmbedder { model, tokenizer, device }))
            .map_err(|_| anyhow::anyhow!("Embedder déjà initialisé"))?;

        tracing::info!("✅ Embedder initialisé (candle BERT, {} dims)", EMBEDDING_DIMS);
        Ok(())
    }

    fn embed_batch(texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let embedder = EMBEDDER
            .get()
            .ok_or_else(|| anyhow::anyhow!("Embedder non initialisé"))?;
        let guard = embedder
            .lock()
            .map_err(|e| anyhow::anyhow!("Lock empoisonné: {}", e))?;

        let mut all_embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let encoding = guard.tokenizer.encode(text.as_str(), true)
                .map_err(|e| anyhow::anyhow!("Tokenization: {}", e))?;

            let ids = encoding.get_ids().to_vec();
            let mask = encoding.get_attention_mask().to_vec();
            let type_ids = encoding.get_type_ids().to_vec();
            let len = ids.len();

            let input_ids = Tensor::new(ids, &guard.device)?.unsqueeze(0)?;
            let attention_mask = Tensor::new(mask, &guard.device)?.unsqueeze(0)?;
            let token_type_ids = Tensor::new(type_ids, &guard.device)?.unsqueeze(0)?;

            let output = guard.model.forward(&input_ids, &token_type_ids, Some(&attention_mask))?;

            // Mean pooling sur les tokens
            let mask_f = attention_mask.to_dtype(candle_core::DType::F32)?
                .unsqueeze(2)?
                .broadcast_as(output.shape())?;
            let masked = (output * mask_f)?;
            let summed = masked.sum(1)?;
            let count = Tensor::new(vec![len as f32], &guard.device)?
                .unsqueeze(0)?
                .broadcast_as(summed.shape())?;
            let mean_pooled = (summed / count)?;

            // Normaliser L2
            let norm = mean_pooled.sqr()?.sum(1)?.sqrt()?
                .unsqueeze(1)?
                .broadcast_as(mean_pooled.shape())?;
            let normalized = (mean_pooled / norm)?;

            let embedding: Vec<f32> = normalized.squeeze(0)?.to_vec1()?;
            all_embeddings.push(embedding);
        }

        Ok(all_embeddings)
    }

    pub fn embed_query(text: &str) -> Result<Vec<f32>> {
        let prefixed = format!("query: {}", text);
        let results = embed_batch(&[prefixed])?;
        results.into_iter().next().ok_or_else(|| anyhow::anyhow!("Aucun embedding"))
    }

    pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let prefixed: Vec<String> = texts.iter().map(|t| format!("passage: {}", t)).collect();
        embed_batch(&prefixed)
    }
}

// ═══════════════════════════════════════════════════════════════════
// API publique — délègue au backend actif
// ═══════════════════════════════════════════════════════════════════

pub fn init_embedder(models_dir: &Path) -> Result<()> {
    backend::init_embedder(models_dir)
}

pub fn embed_query(text: &str) -> Result<Vec<f32>> {
    backend::embed_query(text)
}

pub fn embed_passages(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    backend::embed_passages(texts)
}
