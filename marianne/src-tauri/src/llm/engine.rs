// src-tauri/src/llm/engine.rs
use crate::llm::model::{LoadedModel, ModelConfig};
use crate::llm::tokenizer::MariTokenizer;
use anyhow::{Context, Result};
use std::path::Path;

/// Moteur LLM principal — encapsule le modèle + tokenizer + sampling
pub struct LlmEngine {
    pub model: LoadedModel,
    pub tokenizer: MariTokenizer,
}

impl LlmEngine {
    /// Charger le moteur complet (modèle GGUF + tokenizer)
    pub fn load(models_dir: &Path) -> Result<Self> {
        let model_path = models_dir.join("phi-3-mini-q4.gguf");
        let tokenizer_path = models_dir.join("tokenizer.json");

        if !model_path.exists() {
            anyhow::bail!(
                "Modèle introuvable : {:?}. Lancez le téléchargement d'abord.",
                model_path
            );
        }
        if !tokenizer_path.exists() {
            anyhow::bail!(
                "Tokenizer introuvable : {:?}. Lancez le téléchargement d'abord.",
                tokenizer_path
            );
        }

        let config = ModelConfig::default();
        let model = LoadedModel::from_gguf(&model_path, config)
            .context("Échec du chargement du modèle Phi-3")?;

        let tokenizer = MariTokenizer::load(&tokenizer_path)
            .context("Échec du chargement du tokenizer")?;

        Ok(Self { model, tokenizer })
    }

    /// Générer une réponse en streaming avec callback par token
    ///
    /// Le callback retourne `true` pour continuer, `false` pour arrêter.
    pub fn generate_streaming<F>(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> bool,
    {
        let tokens = self.tokenizer.encode(prompt)?;
        let mut all_tokens = tokens.clone();
        let mut generated_text = String::new();

        // TODO Phase 2 : implémenter la boucle d'inférence Candle
        // Pour l'instant, stub qui retourne un placeholder
        let _ = max_tokens;
        let placeholder = "Je suis Marianne, votre assistante administrative. \
                          Le moteur LLM sera opérationnel dans la Phase 2.";

        for word in placeholder.split_whitespace() {
            let token = format!("{} ", word);
            generated_text.push_str(&token);
            if !on_token(&token) {
                break;
            }
        }

        Ok(generated_text)
    }

    /// Génération bloquante (sans streaming) pour les évaluations internes
    pub fn generate_blocking(&mut self, prompt: &str, max_tokens: usize) -> Result<String> {
        self.generate_streaming(prompt, max_tokens, |_| true)
    }
}
