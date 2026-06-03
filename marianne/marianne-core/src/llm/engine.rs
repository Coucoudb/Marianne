// src-tauri/src/llm/engine.rs
// Moteur LLM basé sur llama.cpp via llama-cpp-2
use crate::profile::{DevicePreference, GpuSelection};
use anyhow::{Context, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::params::LlamaSplitMode;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use std::num::NonZeroU32;
use std::path::Path;
use std::pin::pin;

/// Détecte si un GPU est présent au niveau système (Windows uniquement)
/// Retourne un hint pour aider le diagnostic
#[cfg(target_os = "windows")]
fn detect_system_gpu_hint() -> Option<String> {
    use std::process::Command;
    
    // Essayer wmic pour détecter les cartes graphiques
    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "name"])
        .output()
        .ok()?;
    
    let text = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.eq_ignore_ascii_case("name"))
        .collect();
    
    if lines.is_empty() {
        return None;
    }
    
    // Identifier le type de GPU
    let gpus = lines.join(", ");
    if gpus.to_lowercase().contains("nvidia") {
        Some(format!("GPU NVIDIA détecté: {}", gpus))
    } else if gpus.to_lowercase().contains("amd") || gpus.to_lowercase().contains("radeon") {
        Some(format!("GPU AMD détecté: {}", gpus))
    } else if gpus.to_lowercase().contains("intel") {
        Some(format!("GPU Intel détecté: {}", gpus))
    } else {
        Some(format!("GPU détecté: {}", gpus))
    }
}

#[cfg(not(target_os = "windows"))]
fn detect_system_gpu_hint() -> Option<String> {
    // Sur Linux/macOS, on pourrait utiliser lspci ou system_profiler
    // Mais pour l'instant on laisse None (pas prioritaire)
    None
}

/// Séquences textuelles qui indiquent la fin de la réponse
const STOP_SEQUENCES: &[&str] = &[
    "<|end|>",
    "<|user|>",
    "<|endoftext|>",
    "-----",
    "\nInstruction",
    "\n---\n",
];

/// Configuration du moteur
pub struct EngineConfig {
    pub context_length: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub repeat_penalty: f32,
    pub repeat_last_n: u32,
    pub n_gpu_layers: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            context_length: 4096,
            temperature: 0.15,
            top_p: 0.9,
            repeat_penalty: 1.15,
            repeat_last_n: 64,
            n_gpu_layers: 999,
        }
    }
}

/// Moteur LLM principal — encapsule llama.cpp backend + model
pub struct LlmEngine {
    backend: LlamaBackend,
    model: LlamaModel,
    config: EngineConfig,
}

// Safety: LlamaBackend et LlamaModel sont thread-safe via leur implémentation interne
unsafe impl Send for LlmEngine {}
unsafe impl Sync for LlmEngine {}

impl LlmEngine {
    /// Charger le moteur complet (modèle GGUF via llama.cpp)
    pub fn load(
        models_dir: &Path,
        device_preference: &DevicePreference,
        gpu_selection: &GpuSelection,
        model_filename: &str,
    ) -> Result<Self> {
        let model_path = models_dir.join(model_filename);

        if !model_path.exists() {
            anyhow::bail!(
                "Modèle introuvable : {}. Lancez le téléchargement d'abord.",
                model_path.file_name().and_then(|n| n.to_str()).unwrap_or("fichier")
            );
        }

        let model_name = model_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("modèle");
        tracing::info!("Chargement de {} depuis {:?}", model_name, model_path);

        // Initialiser le backend llama.cpp
        let backend = LlamaBackend::init().context("Échec de l'initialisation llama.cpp")?;

        // Rediriger les logs llama.cpp vers tracing
        llama_cpp_2::send_logs_to_tracing(
            llama_cpp_2::LogOptions::default().with_logs_enabled(false),
        );

        // Configurer les paramètres du modèle
        let config = EngineConfig::default();

        // Détection runtime des devices GPU disponibles
        let all_devices = llama_cpp_2::list_llama_ggml_backend_devices();
        let gpu_devices: Vec<_> = all_devices
            .iter()
            .filter(|d| {
                matches!(
                    d.device_type,
                    llama_cpp_2::LlamaBackendDeviceType::Gpu
                        | llama_cpp_2::LlamaBackendDeviceType::IntegratedGpu
                        | llama_cpp_2::LlamaBackendDeviceType::Accelerator
                )
            })
            .collect();

        let has_gpu = !gpu_devices.is_empty();

        // Afficher les GPU détectés
        if has_gpu {
            for dev in &gpu_devices {
                tracing::info!(
                    "🎮 GPU détecté : {} ({:?}, {} Mo VRAM)",
                    dev.description,
                    dev.device_type,
                    dev.memory_free / 1_048_576,
                );
            }
        } else {
            tracing::warn!("💻 Aucun backend GPU détecté par llama.cpp");
            
            // Diagnostic : détection système des GPU Windows
            #[cfg(target_os = "windows")]
            if let Some(hint) = detect_system_gpu_hint() {
                tracing::warn!(
                    "⚠️ Matériel GPU détecté ({}) mais llama.cpp ne le voit pas.",
                    hint
                );
                tracing::warn!(
                    "   Cause probable : llama.cpp compilé sans support GPU (cuda/vulkan)."
                );
                tracing::warn!("   Solution :");
                tracing::warn!("   • GPU NVIDIA RTX → Recompilez avec: cargo build --release --features cuda");
                tracing::warn!("   • GPU AMD/Intel/Autre → Recompilez avec: cargo build --release --features vulkan");
            }
        }

        let n_gpu_layers = match device_preference {
            DevicePreference::Cpu => {
                tracing::info!("💻 Mode CPU forcé par préférence utilisateur");
                0
            }
            DevicePreference::Gpu if has_gpu => {
                tracing::info!("🚀 GPU disponible — offloading {} couches", config.n_gpu_layers);
                config.n_gpu_layers
            }
            DevicePreference::Gpu => {
                tracing::warn!("💻 Préférence GPU demandée, mais aucun GPU détecté — fallback CPU");
                0
            }
        };

        // Configurer main_gpu et split_mode selon la sélection
        let (main_gpu, split_mode) = if n_gpu_layers > 0 && gpu_devices.len() > 1 {
            match gpu_selection {
                GpuSelection::Auto => {
                    tracing::info!("🎮 GPU Auto — utilisation du GPU principal (index 0)");
                    (0i32, LlamaSplitMode::None)
                }
                GpuSelection::Specific(idx) => {
                    let idx = *idx;
                    if (idx as usize) < gpu_devices.len() {
                        tracing::info!(
                            "🎮 GPU sélectionné : index {} ({})",
                            idx,
                            gpu_devices[idx as usize].description
                        );
                    } else {
                        tracing::warn!(
                            "⚠️ GPU index {} invalide (max: {}), fallback index 0",
                            idx,
                            gpu_devices.len() - 1
                        );
                    }
                    (idx.min((gpu_devices.len() as i32) - 1), LlamaSplitMode::None)
                }
                GpuSelection::AllGpus => {
                    tracing::info!(
                        "🎮 Multi-GPU activé — répartition sur {} GPU (mode Layer)",
                        gpu_devices.len()
                    );
                    (0i32, LlamaSplitMode::Layer)
                }
            }
        } else {
            (0i32, LlamaSplitMode::None)
        };

        let model_params = pin!(LlamaModelParams::default()
            .with_n_gpu_layers(n_gpu_layers)
            .with_main_gpu(main_gpu)
            .with_split_mode(split_mode));

        let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
            .map_err(|e| {
                let err_msg = format!("{:?}", e);
                let mut hint = String::from("Erreur chargement modèle");
                
                // Diagnostic hint: VRAM insuffisante
                if err_msg.contains("OUT_OF_MEMORY") 
                    || err_msg.contains("out of memory") 
                    || err_msg.contains("OutOfMemory") {
                    hint.push_str("\n💡 Conseil : VRAM GPU insuffisante. Essayez :");
                    hint.push_str("\n   • Réduire le nombre de couches GPU (n_gpu_layers)");
                    hint.push_str("\n   • Utiliser un modèle quantifié plus léger (Q4_K_M au lieu de Q6_K)");
                    hint.push_str("\n   • Basculer en mode CPU dans les préférences");
                }
                // Diagnostic hint: Fichier corrompu ou format invalide
                else if err_msg.contains("invalid") 
                    || err_msg.contains("magic") 
                    || err_msg.contains("corrupt") {
                    hint.push_str("\n💡 Conseil : Le fichier GGUF semble corrompu ou invalide. Essayez :");
                    hint.push_str("\n   • Re-télécharger le modèle");
                    hint.push_str("\n   • Vérifier l'intégrité du fichier (checksum)");
                    hint.push_str("\n   • S'assurer que le format est compatible (GGUF uniquement)");
                }
                // Diagnostic hint: Permission ou fichier inaccessible
                else if err_msg.contains("permission") 
                    || err_msg.contains("access") 
                    || err_msg.contains("locked") {
                    hint.push_str("\n💡 Conseil : Impossible d'accéder au fichier. Vérifiez :");
                    hint.push_str("\n   • Les permissions du fichier");
                    hint.push_str("\n   • Qu'aucun autre processus n'utilise le modèle");
                    hint.push_str("\n   • Que le chemin est correct et accessible");
                }
                
                anyhow::anyhow!("{} : {:?}\n{}", hint, e, if hint.contains("💡") { "" } else { "" })
            })?;

        let device_label = if n_gpu_layers > 0 && has_gpu { "GPU" } else { "CPU" };
        let size_mb = std::fs::metadata(&model_path)
            .map(|m| m.len() / 1_048_576)
            .unwrap_or(0);
        let vocab_size = model.n_vocab();

        tracing::info!(
            "✅ {} chargé ({} Mo, {}) — vocab: {} tokens",
            model_name,
            size_mb,
            device_label,
            vocab_size
        );

        Ok(Self {
            backend,
            model,
            config,
        })
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
        // 1. Créer un contexte d'inférence
        let ctx_params =
            LlamaContextParams::default().with_n_ctx(NonZeroU32::new(self.config.context_length));

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| {
                let err_msg = format!("{:?}", e);
                let mut hint = String::from("Erreur création contexte");
                
                // Diagnostic hint: VRAM insuffisante pour le contexte
                if err_msg.contains("OUT_OF_MEMORY") 
                    || err_msg.contains("out of memory") 
                    || err_msg.contains("OutOfMemory")
                    || err_msg.contains("allocation") {
                    hint.push_str("\n💡 Conseil : VRAM insuffisante pour créer le contexte. Essayez :");
                    hint.push_str(&format!("\n   • Réduire la taille du contexte (actuellement {} tokens)", self.config.context_length));
                    hint.push_str("\n   • Réduire le nombre de couches GPU (n_gpu_layers)");
                    hint.push_str("\n   • Libérer de la VRAM (fermer d'autres applications GPU)");
                    hint.push_str("\n   • Basculer en mode CPU dans les préférences");
                }
                
                anyhow::anyhow!("{} : {:?}", hint, e)
            })?;

        // 2. Tokeniser le prompt
        let tokens_list = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| anyhow::anyhow!("Erreur tokenisation : {:?}", e))?;

        let prompt_len = tokens_list.len();
        tracing::debug!(
            "Prompt encodé : {} tokens (max génération: {})",
            prompt_len,
            max_tokens
        );

        if prompt_len >= self.config.context_length as usize {
            anyhow::bail!(
                "Le prompt ({} tokens) dépasse la fenêtre de contexte ({} tokens)",
                prompt_len,
                self.config.context_length
            );
        }

        // 3. Configurer le sampler (température + top-p + pénalité de répétition)
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(
                self.config.repeat_last_n as i32,
                self.config.repeat_penalty,
                0.0, // frequency penalty
                0.0, // presence penalty
            ),
            LlamaSampler::top_p(self.config.top_p, 1),
            LlamaSampler::temp(self.config.temperature),
            LlamaSampler::dist(1234),
        ]);

        // 4. Phase de prefill — encoder le prompt
        let mut batch = LlamaBatch::new(prompt_len.max(512), 1);

        for (i, &token) in tokens_list.iter().enumerate() {
            let is_last = i == prompt_len - 1;
            batch
                .add(token, i as i32, &[0], is_last)
                .context("Erreur ajout token au batch")?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| {
                let err_msg = format!("{:?}", e);
                let mut hint = String::from("Erreur prefill (encodage du prompt)");
                
                // Diagnostic hint: Débordement de contexte
                if err_msg.contains("context") 
                    || err_msg.contains("overflow") 
                    || err_msg.contains("exceed") {
                    hint.push_str(&format!("\n💡 Conseil : Le prompt ({} tokens) est trop long. Essayez :", prompt_len));
                    hint.push_str(&format!("\n   • Réduire la longueur du prompt (max: {} tokens)", self.config.context_length));
                    hint.push_str("\n   • Augmenter la taille du contexte dans la configuration");
                    hint.push_str("\n   • Résumer le contenu RAG avant injection");
                }
                
                anyhow::anyhow!("{} : {:?}", hint, e)
            })?;

        tracing::info!("Premier token généré (prefill terminé)");

        // 5. Boucle autoregressive de génération
        let mut generated_text = String::new();
        let mut generated_count = 0usize;
        let mut n_cur = prompt_len as i32;
        let mut watchdog = super::watchdog::GenerationWatchdog::new();
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        let eos_token = self.model.token_eos();

        for _ in 0..max_tokens {
            // Sampler : choisir le prochain token
            let new_token = sampler.sample(&ctx, -1);

            // Vérifier fin de séquence
            if new_token == eos_token {
                tracing::debug!("EOS détecté après {} tokens", generated_count);
                break;
            }

            // Décoder le token en texte (special=true pour décoder les tokens de contrôle)
            let token_str = match self.model.token_to_piece(new_token, &mut decoder, true, None) {
                Ok(s) => s,
                Err(_) => continue, // token inconnu — ignorer
            };

            // Vérifier les stop sequences
            generated_text.push_str(&token_str);
            let mut stopped = false;
            for stop_seq in STOP_SEQUENCES {
                if generated_text.contains(stop_seq) {
                    if let Some(pos) = generated_text.find(stop_seq) {
                        generated_text.truncate(pos);
                    }
                    stopped = true;
                    break;
                }
            }

            if stopped {
                tracing::debug!("Stop sequence détectée après {} tokens", generated_count);
                break;
            }

            generated_count += 1;

            // Watchdog : vérifier les boucles de répétition
            match watchdog.check(&token_str) {
                super::watchdog::WatchdogStatus::Continue => {}
                super::watchdog::WatchdogStatus::Abort(reason) => {
                    tracing::warn!("Génération interrompue par watchdog : {}", reason);
                    break;
                }
            }

            // Callback streaming — arrêt si false
            if !on_token(&token_str) {
                tracing::debug!(
                    "Génération interrompue par callback à {} tokens",
                    generated_count
                );
                break;
            }

            // Préparer le batch pour le prochain token
            batch.clear();
            batch
                .add(new_token, n_cur, &[0], true)
                .context("Erreur ajout token au batch")?;
            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| {
                    let err_msg = format!("{:?}", e);
                    let mut hint = String::from("Erreur decode (génération de token)");
                    
                    // Diagnostic hint: Débordement de contexte en génération
                    if err_msg.contains("context") 
                        || err_msg.contains("overflow") 
                        || err_msg.contains("exceed") {
                        let total_tokens = n_cur as usize;
                        hint.push_str(&format!("\n💡 Conseil : Débordement du contexte ({} tokens utilisés sur {}). Essayez :", 
                            total_tokens, self.config.context_length));
                        hint.push_str("\n   • Réduire max_tokens pour la génération");
                        hint.push_str("\n   • Augmenter context_length dans la configuration");
                        hint.push_str("\n   • Utiliser un prompt plus court");
                    }
                    
                    anyhow::anyhow!("{} : {:?}", hint, e)
                })?;
        }

        tracing::info!("Génération terminée : {} tokens produits", generated_count);

        // Valider la réponse
        match watchdog.validate_response(&generated_text) {
            super::watchdog::ResponseValidity::Valid => Ok(generated_text),
            super::watchdog::ResponseValidity::TooShort => Ok(
                "Je n'ai pas pu générer une réponse complète. Veuillez reformuler votre question."
                    .to_string(),
            ),
            super::watchdog::ResponseValidity::Garbage => Ok(
                "Une erreur interne s'est produite. Essayez de relancer l'application."
                    .to_string(),
            ),
        }
    }

    /// Génération bloquante (sans streaming) pour les évaluations internes
    pub fn generate_blocking(&mut self, prompt: &str, max_tokens: usize) -> Result<String> {
        self.generate_streaming(prompt, max_tokens, |_| true)
    }
}
