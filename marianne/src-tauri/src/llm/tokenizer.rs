// src-tauri/src/llm/tokenizer.rs
use anyhow::{Context, Result};
use std::path::Path;
use tokenizers::Tokenizer;

/// Wrapper autour du tokenizer HuggingFace
pub struct MariTokenizer {
    inner: Tokenizer,
}

impl MariTokenizer {
    pub fn load(path: &Path) -> Result<Self> {
        let inner = Tokenizer::from_file(path)
            .map_err(|e| anyhow::anyhow!("Erreur tokenizer : {}", e))?;
        Ok(Self { inner })
    }

    /// Encoder un texte en tokens (IDs)
    pub fn encode(&self, text: &str) -> Result<Vec<u32>> {
        let encoding = self
            .inner
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Erreur d'encodage : {}", e))?;
        Ok(encoding.get_ids().to_vec())
    }

    /// Décoder des token IDs en texte
    pub fn decode(&self, tokens: &[u32]) -> Result<String> {
        self.inner
            .decode(tokens, true)
            .map_err(|e| anyhow::anyhow!("Erreur de décodage : {}", e))
    }

    /// Taille du vocabulaire
    pub fn vocab_size(&self) -> usize {
        self.inner.get_vocab_size(true)
    }
}
