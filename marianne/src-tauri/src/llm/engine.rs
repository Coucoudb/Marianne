// src-tauri/src/llm/engine.rs
use crate::llm::model::{LoadedModel, ModelConfig};
use crate::llm::sampler::Sampler;
use crate::llm::tokenizer::MariTokenizer;
use anyhow::{Context, Result};
use candle_core::Tensor;
use std::path::Path;

/// Token ID de fin de séquence pour Phi-3-Mini
const EOS_TOKEN_ID: u32 = 32000;
/// Token ID <|end|> — fin de tour dans le format instruct Phi-3
const END_TOKEN_ID: u32 = 32007;

/// Séquences textuelles qui indiquent la fin de la réponse
const STOP_SEQUENCES: &[&str] = &["<|end|>", "<|user|>", "<|endoftext|>", "-----"];

/// Moteur LLM principal — encapsule le modèle + tokenizer + sampling
pub struct LlmEngine {
    pub model: LoadedModel,
    pub tokenizer: MariTokenizer,
    pub sampler: Sampler,
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
        let sampler = Sampler::new(
            config.temperature,
            config.top_p,
            config.repeat_penalty,
            config.repeat_last_n,
        );

        let model = LoadedModel::from_gguf(&model_path, config)
            .context("Échec du chargement du modèle Phi-3")?;

        let tokenizer = MariTokenizer::load(&tokenizer_path)
            .context("Échec du chargement du tokenizer")?;

        tracing::info!(
            "✅ Moteur LLM prêt — vocab: {} tokens, device: {:?}",
            tokenizer.vocab_size(),
            model.device
        );

        Ok(Self {
            model,
            tokenizer,
            sampler,
        })
    }

    /// Générer une réponse en streaming avec callback par token
    ///
    /// Le callback retourne `true` pour continuer, `false` pour arrêter.
    /// Implémente la boucle autoregressive complète :
    /// encode → forward → sample → decode → callback → repeat
    pub fn generate_streaming<F>(
        &mut self,
        prompt: &str,
        max_tokens: usize,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> bool,
    {
        // 1. Encoder le prompt en tokens
        let prompt_tokens = self.tokenizer.encode(prompt)?;
        let prompt_len = prompt_tokens.len();

        tracing::debug!(
            "Prompt encodé : {} tokens (max génération: {})",
            prompt_len,
            max_tokens
        );

        // Vérifier que le prompt ne dépasse pas la fenêtre de contexte
        if prompt_len >= self.model.config.context_length {
            anyhow::bail!(
                "Le prompt ({} tokens) dépasse la fenêtre de contexte ({} tokens)",
                prompt_len,
                self.model.config.context_length
            );
        }

        // 2. Préparer les tokens pour l'inférence
        let mut all_tokens: Vec<u32> = prompt_tokens;
        let mut generated_text = String::new();
        let mut prev_decoded_len = 0usize;
        let mut generated_count = 0usize;

        // 3. Boucle autoregressive token par token
        // Phase de prefill : on passe tout le prompt d'un coup
        // Phase de decode : on génère un token à la fois
        let mut pos = 0;

        for index in 0..max_tokens {
            // Construire le tensor d'entrée
            let context_size = if index == 0 {
                // Prefill : tout le prompt
                all_tokens.len()
            } else {
                // Decode : un seul token (le dernier généré)
                1
            };

            let start_pos = all_tokens.len().saturating_sub(context_size);
            let input_tokens = &all_tokens[start_pos..];

            let input_tensor = Tensor::new(input_tokens, &self.model.device)?
                .unsqueeze(0)?; // Shape: [1, seq_len]

            // 4. Forward pass dans le modèle
            // quantized_phi3 retourne (batch_size, vocab_size) — il extrait le dernier token en interne
            let logits = self.model.model.forward(&input_tensor, pos)?;
            pos += context_size;

            // 5. Extraire les logits — squeeze le batch dim pour obtenir [vocab_size]
            let last_logits = logits.squeeze(0)?;

            // 6. Sampler : pénalité de répétition + température + top-p
            let next_token = self.sampler.sample(&last_logits, &all_tokens)?;

            // 7. Vérifier fin de séquence
            if next_token == EOS_TOKEN_ID || next_token == END_TOKEN_ID {
                tracing::debug!("EOS/END détecté après {} tokens", generated_count);
                break;
            }

            // 8. Ajouter le token et décoder
            all_tokens.push(next_token);
            generated_count += 1;

            if generated_count == 1 {
                tracing::info!("Premier token généré (prefill terminé)");
            }

            // Décoder TOUS les tokens générés ensemble pour gérer les espaces BPE correctement
            let full_decoded = self.tokenizer.decode(&all_tokens[prompt_len..])?;

            // Vérifier les stop sequences dans le texte décodé
            let mut stopped = false;
            let mut clean_decoded = full_decoded.clone();
            for stop_seq in STOP_SEQUENCES {
                if let Some(pos) = full_decoded.find(stop_seq) {
                    clean_decoded = full_decoded[..pos].to_string();
                    stopped = true;
                    break;
                }
            }

            let new_text = &clean_decoded[prev_decoded_len..];

            if !new_text.is_empty() {
                generated_text = clean_decoded[..].to_string();
                prev_decoded_len = clean_decoded.len();

                // 9. Callback streaming — arrêt si false
                if !on_token(new_text) {
                    tracing::debug!("Génération interrompue par callback à {} tokens", generated_count);
                    break;
                }
            }

            if stopped {
                tracing::debug!("Stop sequence détectée après {} tokens", generated_count);
                break;
            }
        }

        tracing::info!(
            "Génération terminée : {} tokens produits",
            generated_count
        );

        Ok(generated_text)
    }

    /// Génération bloquante (sans streaming) pour les évaluations internes
    pub fn generate_blocking(&mut self, prompt: &str, max_tokens: usize) -> Result<String> {
        self.generate_streaming(prompt, max_tokens, |_| true)
    }
}
