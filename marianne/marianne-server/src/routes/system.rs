// marianne-server/src/routes/system.rs
// GET /api/v1/system/info — Informations système et modèle pour le panneau Settings en mode web.

use crate::state::ServerState;
use axum::{extract::State, Json};
use marianne_core::llm::device::{is_gpu_available, list_backend_devices, LlamaBackendDevice, LlamaBackendDeviceType};
use marianne_core::profile::{DevicePreference, GpuSelection};
use serde::Serialize;

#[derive(Serialize)]
pub struct SystemInfo {
    pub device: DeviceInfo,
    pub model: ModelInfo,
    pub preference: PreferenceInfo,
    pub gpu_devices: Vec<GpuDeviceInfo>,
}

#[derive(Serialize)]
pub struct DeviceInfo {
    /// "gpu" ou "cpu"
    pub backend: String,
    /// Libellé lisible (ex: "GPU (NVIDIA RTX 3060)" ou "CPU (12 threads)")
    pub label: String,
    /// Indique si un GPU est disponible sur cette machine
    pub gpu_available: bool,
}

#[derive(Serialize)]
pub struct ModelInfo {
    /// Nom du modèle actif
    pub name: String,
    /// Indique si le modèle est chargé en mémoire
    pub active: bool,
}

#[derive(Serialize)]
pub struct PreferenceInfo {
    /// Préférence utilisateur (Gpu | Cpu)
    pub device: DevicePreference,
    /// Sélection GPU (Auto | Specific | AllGpus)
    pub gpu_selection: GpuSelection,
}

#[derive(Serialize)]
pub struct GpuDeviceInfo {
    /// Index du GPU (utilisé pour la sélection)
    pub index: i32,
    /// Nom/description du GPU
    pub name: String,
    /// Type de device ("gpu", "integrated_gpu", "accelerator")
    pub device_type: String,
    /// VRAM libre en Mo
    pub vram_free_mb: u64,
}

/// Handler GET /api/v1/system/info
pub async fn get_system_info(State(server): State<ServerState>) -> Json<SystemInfo> {
    // ✅ FIX 1: Appeler llama_cpp backend devices UNE SEULE FOIS
    let backend_devices = list_backend_devices();
    
    let gpu_available = is_gpu_available_from_list(&backend_devices);
    let profile = server.core.profile.lock().clone();

    // ✅ FIX 3: Factoriser la logique dupliquée
    let (backend, label) = resolve_device_info(&profile.device_preference, &backend_devices, gpu_available);

    // ✅ FIX 2: Utiliser tokio::fs pour les I/O
    let model_name = resolve_model_name(&server.core.data_dir, &profile.selected_model).await;

    // Réutiliser la liste des devices
    let gpu_devices = list_gpu_devices(backend_devices);

    Json(SystemInfo {
        device: DeviceInfo {
            backend,
            label,
            gpu_available,
        },
        model: ModelInfo {
            name: model_name,
            active: server.core.is_model_loaded(),
        },
        preference: PreferenceInfo {
            device: profile.device_preference,
            gpu_selection: profile.gpu_selection,
        },
        gpu_devices,
    })
}

// ─── Fonctions utilitaires ────────────────────────────────────────────────────

/// Détecter si un GPU est disponible sur cette machine (version helper interne)
fn is_gpu_available_from_list(devices: &[LlamaBackendDevice]) -> bool {
    devices.iter().any(|d| {
        matches!(
            d.device_type,
            LlamaBackendDeviceType::Gpu
                | LlamaBackendDeviceType::IntegratedGpu
                | LlamaBackendDeviceType::Accelerator
        )
    })
}

/// ✅ FIX 3: Factoriser la logique dupliquée pour résoudre backend/label
fn resolve_device_info(
    preference: &DevicePreference,
    devices: &[LlamaBackendDevice],
    gpu_available: bool,
) -> (String, String) {
    if gpu_available && matches!(preference, DevicePreference::Gpu) {
        let gpu_label = devices
            .iter()
            .find(|d| {
                matches!(
                    d.device_type,
                    LlamaBackendDeviceType::Gpu
                        | LlamaBackendDeviceType::IntegratedGpu
                        | LlamaBackendDeviceType::Accelerator
                )
            })
            .map(|d| d.description.clone())
            .unwrap_or_else(|| "GPU".into());
        ("gpu".into(), format!("GPU ({})", gpu_label))
    } else {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .saturating_sub(1)
            .max(1);
        ("cpu".into(), format!("CPU ({threads} threads)"))
    }
}

/// Lister tous les GPU disponibles sur la machine
fn list_gpu_devices(devices: Vec<LlamaBackendDevice>) -> Vec<GpuDeviceInfo> {
    devices
        .into_iter()
        .filter(|d| {
            matches!(
                d.device_type,
                LlamaBackendDeviceType::Gpu
                    | LlamaBackendDeviceType::IntegratedGpu
                    | LlamaBackendDeviceType::Accelerator
            )
        })
        .enumerate()
        .map(|(idx, d)| {
            let device_type = match d.device_type {
                LlamaBackendDeviceType::Gpu => "gpu",
                LlamaBackendDeviceType::IntegratedGpu => "integrated_gpu",
                LlamaBackendDeviceType::Accelerator => "accelerator",
                _ => "unknown",
            };
            GpuDeviceInfo {
                index: idx as i32,
                name: d.description.clone(),
                device_type: device_type.to_string(),
                vram_free_mb: (d.memory_free / 1_048_576) as u64,
            }
        })
        .collect()
}

/// ✅ FIX 2: Résoudre le nom lisible du modèle sélectionné (async avec tokio::fs)
async fn resolve_model_name(data_dir: &std::path::Path, selected_model: &str) -> String {
    // Charger le registre des modèles installés
    let registry_path = data_dir.join("models").join("registry.json");
    let installed: Vec<InstalledModel> = tokio::fs::read_to_string(&registry_path)
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    installed
        .iter()
        .find(|m| m.id == selected_model)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| {
            // Fallback : noms par défaut
            match selected_model {
                "phi-3-mini-q4" => "Phi-3 Mini (Q4)".to_string(),
                "phi-3.5-mini-q4" => "Phi-3.5 Mini (Q4)".to_string(),
                "phi-3-medium-q4" => "Phi-3 Medium (Q4)".to_string(),
                _ => selected_model.to_string(),
            }
        })
}

/// Structure du registre des modèles installés (réutilisée de src-tauri)
#[derive(serde::Deserialize)]
struct InstalledModel {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub repo_id: String,
    #[allow(dead_code)]
    pub filename: String,
    #[allow(dead_code)]
    pub size_mb: u64,
}
