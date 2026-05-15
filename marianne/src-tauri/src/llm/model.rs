// src-tauri/src/llm/model.rs
use anyhow::{Context, Result};
use candle_core::Device;
use candle_transformers::models::quantized_phi3::ModelWeights as Phi3;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub context_length: usize,
    /// Faible pour du droit/admin (déterminisme > créativité)
    pub temperature: f64,
    pub top_p: f64,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            context_length: 4096,
            temperature: 0.15,
            top_p: 0.9,
            repeat_penalty: 1.15,
            repeat_last_n: 64,
        }
    }
}

pub struct LoadedModel {
    pub model: Phi3,
    pub config: ModelConfig,
    pub device: Device,
}

/// Détecter et retourner le meilleur device disponible.
///
/// Ordre de priorité :
/// 1. CUDA (Nvidia) — si feature "cuda" activée et GPU présent
/// 2. Metal (Apple Silicon) — si feature "metal" activée et sur Mac ARM
/// 3. CPU — toujours disponible, fallback garanti
pub fn get_best_device() -> candle_core::Result<Device> {
    #[cfg(feature = "cuda")]
    {
        if let Ok(device) = Device::new_cuda(0) {
            tracing::info!("🚀 GPU Nvidia détecté — utilisation de CUDA");
            return Ok(device);
        }
    }

    #[cfg(feature = "metal")]
    {
        if let Ok(device) = Device::new_metal(0) {
            tracing::info!("🍏 Apple Silicon détecté — utilisation de Metal");
            return Ok(device);
        }
    }

    let n_threads = num_cpus::get().saturating_sub(1).max(1);
    std::env::set_var("RAYON_NUM_THREADS", n_threads.to_string());
    tracing::info!("💻 CPU mode — {} threads alloués à Marianne", n_threads);

    Ok(Device::Cpu)
}

impl LoadedModel {
    pub fn from_gguf(model_path: &Path, config: ModelConfig) -> Result<Self> {
        tracing::info!("Chargement Phi-3-Mini depuis {:?}", model_path);

        let device = get_best_device()?;

        let mut file = std::fs::File::open(model_path)
            .with_context(|| format!("Impossible d'ouvrir {:?}", model_path))?;

        let gguf_content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .context("Erreur de lecture GGUF")?;

        let model = Phi3::from_gguf(false, gguf_content, &mut file, &device)
            .context("Erreur de chargement des poids Phi-3")?;

        let size_mb = std::fs::metadata(model_path)?.len() / 1_048_576;
        tracing::info!("✅ Phi-3-Mini chargé ({} Mo)", size_mb);

        Ok(Self {
            model,
            config,
            device,
        })
    }
}
