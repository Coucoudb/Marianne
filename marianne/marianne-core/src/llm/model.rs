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
            .with_context(|| {
                format!("Impossible d'ouvrir {}\n💡 Conseil : Vérifiez que le fichier existe et que vous avez les permissions de lecture.", 
                    model_path.file_name().and_then(|n| n.to_str()).unwrap_or("le modèle"))
            })?;

        let gguf_content = candle_core::quantized::gguf_file::Content::read(&mut file)
            .map_err(|e| {
                let err_msg = format!("{:?}", e);
                let mut hint = String::from("Erreur de lecture GGUF");
                
                if err_msg.contains("magic") || err_msg.contains("invalid") {
                    hint.push_str("\n💡 Conseil : Le fichier GGUF semble invalide ou corrompu. Essayez :");
                    hint.push_str("\n   • Re-télécharger le modèle");
                    hint.push_str("\n   • Vérifier le format (doit être GGUF)");
                }
                
                anyhow::anyhow!("{}: {:?}", hint, e)
            })?;

        // Tenter le chargement sur le device choisi, fallback CPU si OOM GPU
        let (final_model, final_device) = match Phi3::from_gguf(false, gguf_content, &mut file, &device) {
            Ok(model) => (model, device),
            Err(e) if !matches!(device, Device::Cpu) => {
                let err_msg = format!("{:?}", e);
                if err_msg.contains("OUT_OF_MEMORY") || err_msg.contains("out of memory") || err_msg.contains("OutOfMemory") {
                    tracing::warn!("⚠ Mémoire GPU insuffisante pour {} — basculement sur CPU", model_name);
                    tracing::info!("💡 Conseil : Pour utiliser le GPU, essayez un modèle plus léger (Q4_K_M au lieu de Q6_K)");
                    // Relire le fichier pour un second essai
                    let mut file2 = std::fs::File::open(model_path)?;
                    let gguf2 = candle_core::quantized::gguf_file::Content::read(&mut file2)
                        .context("Erreur de lecture GGUF (retry CPU)")?;
                    let cpu_model = Phi3::from_gguf(false, gguf2, &mut file2, &Device::Cpu)
                        .context("Échec du chargement sur CPU également")?;
                    (cpu_model, Device::Cpu)
                } else {
                    let mut hint = String::from("Erreur de chargement des poids du modèle");
                    
                    if err_msg.contains("tensor") || err_msg.contains("shape") {
                        hint.push_str("\n💡 Conseil : Incompatibilité de structure du modèle. Vérifiez :");
                        hint.push_str("\n   • Que le modèle est compatible Phi-3");
                        hint.push_str("\n   • Que la version de Candle est à jour");
                    } else if err_msg.contains("device") {
                        hint.push_str("\n💡 Conseil : Problème de device GPU. Essayez :");
                        hint.push_str("\n   • Vérifier les drivers GPU");
                        hint.push_str("\n   • Basculer en mode CPU");
                    }
                    
                    return Err(anyhow::anyhow!("{}: {:?}", hint, e));
                }
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                let mut hint = String::from("Erreur de chargement des poids du modèle");
                
                if err_msg.contains("allocation") || err_msg.contains("memory") {
                    hint.push_str("\n💡 Conseil : Mémoire système insuffisante. Essayez :");
                    hint.push_str("\n   • Fermer d'autres applications");
                    hint.push_str("\n   • Utiliser un modèle quantifié plus léger");
                }
                
                return Err(anyhow::anyhow!("{}: {:?}", hint, e));
            }
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
