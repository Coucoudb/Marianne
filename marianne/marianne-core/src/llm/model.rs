// src-tauri/src/llm/model.rs
use anyhow::{Context, Result};
use candle_core::Device;
use candle_transformers::models::quantized_phi3::ModelWeights as Phi3;
use crate::profile::DevicePreference;
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
///
/// Si `preference` est `Cpu`, force le mode CPU même si un GPU est disponible.
pub fn get_best_device(preference: &DevicePreference) -> candle_core::Result<Device> {
    if *preference == DevicePreference::Cpu {
        let n_threads = num_cpus::get().saturating_sub(1).max(1);
        std::env::set_var("RAYON_NUM_THREADS", n_threads.to_string());
        tracing::info!("💻 CPU mode (préférence utilisateur) — {} threads alloués", n_threads);
        return Ok(Device::Cpu);
    }

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
    pub fn from_gguf(model_path: &Path, config: ModelConfig, device_preference: &DevicePreference) -> Result<Self> {
        let model_name = model_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("modèle");
        tracing::info!("Chargement de {} depuis {:?}", model_name, model_path);

        let device = get_best_device(device_preference)?;

        let mut file = std::fs::File::open(model_path)
            .with_context(|| format!("Impossible d'ouvrir {:?}", model_path))?;

        let gguf_content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .context("Erreur de lecture GGUF")?;

        // Tenter le chargement sur le device choisi, fallback CPU si OOM GPU
        let (final_model, final_device) = match Phi3::from_gguf(false, gguf_content, &mut file, &device) {
            Ok(model) => (model, device),
            Err(e) if !matches!(device, Device::Cpu) => {
                let err_msg = format!("{:?}", e);
                if err_msg.contains("OUT_OF_MEMORY") || err_msg.contains("out of memory") || err_msg.contains("OutOfMemory") {
                    tracing::warn!("⚠ Mémoire GPU insuffisante pour {} — basculement sur CPU", model_name);
                    // Relire le fichier pour un second essai
                    let mut file2 = std::fs::File::open(model_path)?;
                    let gguf2 = candle_core::quantized::gguf_file::Content::read(&mut file2)
                        .context("Erreur de lecture GGUF (retry CPU)")?;
                    let cpu_model = Phi3::from_gguf(false, gguf2, &mut file2, &Device::Cpu)
                        .context("Échec du chargement sur CPU également")?;
                    (cpu_model, Device::Cpu)
                } else {
                    return Err(e).context("Erreur de chargement des poids du modèle");
                }
            }
            Err(e) => return Err(e).context("Erreur de chargement des poids du modèle"),
        };

        let size_mb = std::fs::metadata(model_path)?.len() / 1_048_576;
        let device_label = if matches!(final_device, Device::Cpu) {
            "CPU"
        } else {
            "GPU"
        };
        tracing::info!("✅ {} chargé ({} Mo, {})", model_name, size_mb, device_label);

        Ok(Self {
            model: final_model,
            config,
            device: final_device,
        })
    }
}
