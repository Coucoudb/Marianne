// src-tauri/src/commands/setup.rs
use crate::profile::{DevicePreference, GpuSelection};
use crate::state::AppState;
use futures_util::StreamExt;
use reqwest::Client;
use std::io::{Seek, SeekFrom, Write};
use tauri::{Emitter, State, Window};

#[derive(serde::Serialize, Clone)]
pub struct ModelStatus {
    pub model_downloaded: bool,
    pub tokenizer_downloaded: bool,
    pub model_loaded: bool,
    pub model_size_mb: u64,
    pub available_ram_gb: f32,
}

#[derive(serde::Serialize, Clone)]
struct DownloadProgress {
    filename: String,
    downloaded_mb: u64,
    total_mb: u64,
    percent: u32,
}

#[derive(serde::Serialize, Clone)]
pub struct DeviceInfo {
    /// "cuda", "metal", ou "cpu"
    pub backend: String,
    /// Libellé lisible, ex. "GPU CUDA" ou "CPU (12 threads)"
    pub label: String,
    /// Indique si un GPU est disponible sur cette machine
    pub gpu_available: bool,
}

/// Retourner le device utilisé par le moteur LLM
#[tauri::command]
pub async fn get_device_info(state: State<'_, AppState>) -> Result<DeviceInfo, String> {
    let gpu_available = is_gpu_available();
    let guard = state.llm.lock();
    match guard.as_ref() {
        Some(_engine) => {
            // Avec llama.cpp, le device est déterminé par la config n_gpu_layers
            let profile = state.profile.lock();
            let (backend, label) = match profile.device_preference {
                DevicePreference::Gpu if gpu_available => {
                    // Déterminer le type de backend GPU
                    let gpu_label = llama_cpp_2::list_llama_ggml_backend_devices()
                        .iter()
                        .find(|d| matches!(
                            d.device_type,
                            llama_cpp_2::LlamaBackendDeviceType::Gpu
                                | llama_cpp_2::LlamaBackendDeviceType::IntegratedGpu
                                | llama_cpp_2::LlamaBackendDeviceType::Accelerator
                        ))
                        .map(|d| d.description.clone())
                        .unwrap_or_else(|| "GPU".into());
                    ("gpu".into(), format!("GPU ({})", gpu_label))
                }
                _ => {
                    let threads = num_cpus::get().saturating_sub(1).max(1);
                    ("cpu".into(), format!("CPU ({threads} threads)"))
                }
            };
            Ok(DeviceInfo { backend, label, gpu_available })
        }
        None => Err("Modèle non chargé".into()),
    }
}

/// Vérifier l'état du modèle et du système
#[tauri::command]
pub async fn check_model_status(state: State<'_, AppState>) -> Result<ModelStatus, String> {
    let selected_model = state.profile.lock().selected_model.clone();
    let model_filename = resolve_model_filename(&state.data_dir, &selected_model);

    let model_path = state.data_dir.join("models").join(&model_filename);
    let available_ram_gb = read_available_ram_gb();

    Ok(ModelStatus {
        model_downloaded: model_path.exists(),
        tokenizer_downloaded: true, // llama.cpp a un tokenizer intégré
        model_loaded: state.is_model_loaded(),
        model_size_mb: if model_path.exists() {
            std::fs::metadata(&model_path)
                .map(|m| m.len() / 1_048_576)
                .unwrap_or(0)
        } else {
            0
        },
        available_ram_gb,
    })
}

/// Télécharger le modèle par défaut avec reprise automatique via HTTP Range
#[tauri::command]
pub async fn download_model(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let models_dir = state.data_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        DEFAULT_MODEL_REPO, DEFAULT_MODEL_FILE
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .map_err(|e| e.to_string())?;

    download_file_with_resume(&client, &window, &download_url, DEFAULT_MODEL_FILE, &models_dir).await?;

    // Enregistrer dans le registre local
    let size_mb = models_dir
        .join(DEFAULT_MODEL_FILE)
        .metadata()
        .map(|m| m.len() / 1_048_576)
        .unwrap_or(0);

    let mut installed = load_installed_models(&state.data_dir);
    installed.retain(|m| m.id != DEFAULT_MODEL_ID);
    installed.push(InstalledModel {
        id: DEFAULT_MODEL_ID.to_string(),
        repo_id: DEFAULT_MODEL_REPO.to_string(),
        filename: DEFAULT_MODEL_FILE.to_string(),
        name: "Phi-3 Mini (Q4)".to_string(),
        size_mb,
    });
    save_installed_models(&state.data_dir, &installed)?;

    // Sélectionner le modèle par défaut
    {
        let mut profile = state.profile.lock();
        profile.selected_model = DEFAULT_MODEL_ID.to_string();
        profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Charger le modèle sélectionné en mémoire
#[tauri::command]
pub async fn load_model(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _ = window.emit("model-loading", "Chargement du modèle en mémoire...");

    // Libérer l'ancien modèle d'abord (libère la VRAM)
    {
        let mut guard = state.llm.lock();
        if guard.is_some() {
            *guard = None;
            tracing::info!("Ancien modèle déchargé (libération mémoire)");
        }
    }

    let models_dir = state.data_dir.join("models");
    let profile = state.profile.lock().clone();
    let device_preference = profile.device_preference.clone();
    let gpu_selection = profile.gpu_selection.clone();
    let selected_model = profile.selected_model.clone();

    // Résoudre le nom de fichier GGUF à partir du registre
    let model_filename = resolve_model_filename(&state.data_dir, &selected_model);

    let engine = tokio::task::spawn_blocking(move || {
        crate::llm::engine::LlmEngine::load(&models_dir, &device_preference, &gpu_selection, &model_filename)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    *state.llm.lock() = Some(engine);

    let _ = window.emit("model-ready", "Marianne est prête !");
    tracing::info!("✅ Modèle {} chargé en mémoire", selected_model);
    Ok(())
}

/// Initialiser le RAG (ingestion du corpus légal)
#[tauri::command]
pub async fn initialize_rag(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _ = window.emit("rag-loading", "Indexation du corpus légal...");

    let corpus_dir = state.data_dir.join("corpus");
    let models_dir = state.data_dir.join("models");

    if !corpus_dir.exists() {
        tracing::info!("Création du répertoire corpus : {:?}", corpus_dir);
        std::fs::create_dir_all(&corpus_dir).map_err(|e| e.to_string())?;
    }

    // Seed : copier les fiches bundlées si le corpus est vide
    seed_corpus_from_resources(&window, &corpus_dir);

    let store = state.vector_store.clone();
    let chunks = crate::rag::ingestion::ingest_corpus(&corpus_dir, &store, &models_dir)
        .await
        .map_err(|e| e.to_string())?;

    // Charger les hashes des chunks web existants pour déduplication
    match store.load_all_content_hashes().await {
        Ok(hashes) => {
            for h in hashes {
                state.known_hashes.insert(h);
            }
            if !state.known_hashes.is_empty() {
                tracing::info!("Chargé {} hashes web pour déduplication", state.known_hashes.len());
            }
        }
        Err(e) => tracing::debug!("Pas de hashes web à charger : {}", e),
    }

    let _ = window.emit("rag-ready", format!("{} passages indexés", chunks));
    tracing::info!("✅ RAG initialisé : {} chunks", chunks);
    Ok(())
}

fn read_available_ram_gb() -> f32 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemAvailable:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb as f32 / 1_048_576.0;
                        }
                    }
                }
            }
        }
    }
    // Windows/macOS — valeur par défaut si non détectable
    4.0
}

/// Copier les fiches Markdown bundlées dans le corpus si celui-ci est vide.
/// Permet d'avoir un corpus initial dès la première utilisation.
fn seed_corpus_from_resources(window: &Window, corpus_dir: &std::path::Path) {
    use tauri::Manager;

    // Ne rien faire si le corpus contient déjà des .md
    let has_md = std::fs::read_dir(corpus_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        == Some("md")
                })
        })
        .unwrap_or(false);

    if has_md {
        return;
    }

    // Chercher les resources bundlées
    let resource_dir = match window.app_handle().path().resource_dir() {
        Ok(dir) => dir,
        Err(_) => return,
    };

    // Les fichiers bundlés via "resources": ["../corpus/*"] sont copiés à la racine du resource_dir
    let mut copied = 0usize;
    if let Ok(entries) = std::fs::read_dir(&resource_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                let dest = corpus_dir.join(path.file_name().unwrap());
                if !dest.exists() {
                    if std::fs::copy(&path, &dest).is_ok() {
                        copied += 1;
                    }
                }
            }
        }
    }

    if copied > 0 {
        tracing::info!("📚 {} fiches initiales copiées dans le corpus", copied);
    }
}

/// Détecter si un GPU est disponible sur cette machine (runtime)
fn is_gpu_available() -> bool {
    llama_cpp_2::list_llama_ggml_backend_devices()
        .iter()
        .any(|d| {
            matches!(
                d.device_type,
                llama_cpp_2::LlamaBackendDeviceType::Gpu
                    | llama_cpp_2::LlamaBackendDeviceType::IntegratedGpu
                    | llama_cpp_2::LlamaBackendDeviceType::Accelerator
            )
        })
}

/// Récupérer la préférence de device + disponibilité GPU
#[tauri::command]
pub async fn get_device_preference(state: State<'_, AppState>) -> Result<DevicePreferenceInfo, String> {
    let profile = state.profile.lock();
    Ok(DevicePreferenceInfo {
        preference: profile.device_preference.clone(),
        gpu_available: is_gpu_available(),
    })
}

/// Sauvegarder la préférence de device (appliquée au prochain démarrage)
#[tauri::command]
pub async fn set_device_preference(
    state: State<'_, AppState>,
    preference: DevicePreference,
) -> Result<(), String> {
    let mut profile = state.profile.lock();
    profile.device_preference = preference;
    profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize, Clone)]
pub struct DevicePreferenceInfo {
    pub preference: DevicePreference,
    pub gpu_available: bool,
}

// ─── Gestion multi-GPU ─────────────────────────────────────────────────────────

#[derive(serde::Serialize, Clone)]
pub struct GpuDeviceInfo {
    /// Index du GPU (utilisé pour la sélection)
    pub index: i32,
    /// Nom/description du GPU
    pub name: String,
    /// Type de device (gpu, integrated_gpu, accelerator)
    pub device_type: String,
    /// VRAM libre en Mo
    pub vram_free_mb: u64,
}

#[derive(serde::Serialize, Clone)]
pub struct GpuListInfo {
    /// Liste des GPU disponibles
    pub devices: Vec<GpuDeviceInfo>,
    /// Sélection actuelle de l'utilisateur
    pub selection: GpuSelection,
}

/// Lister tous les GPU disponibles sur la machine
#[tauri::command]
pub async fn list_gpu_devices(state: State<'_, AppState>) -> Result<GpuListInfo, String> {
    let devices: Vec<GpuDeviceInfo> = llama_cpp_2::list_llama_ggml_backend_devices()
        .into_iter()
        .enumerate()
        .filter(|(_, d)| {
            matches!(
                d.device_type,
                llama_cpp_2::LlamaBackendDeviceType::Gpu
                    | llama_cpp_2::LlamaBackendDeviceType::IntegratedGpu
                    | llama_cpp_2::LlamaBackendDeviceType::Accelerator
            )
        })
        .map(|(idx, d)| {
            let device_type = match d.device_type {
                llama_cpp_2::LlamaBackendDeviceType::Gpu => "gpu",
                llama_cpp_2::LlamaBackendDeviceType::IntegratedGpu => "integrated_gpu",
                llama_cpp_2::LlamaBackendDeviceType::Accelerator => "accelerator",
                _ => "unknown",
            };
            GpuDeviceInfo {
                index: idx as i32,
                name: d.description.clone(),
                device_type: device_type.to_string(),
                vram_free_mb: (d.memory_free / 1_048_576) as u64,
            }
        })
        .collect();

    let selection = state.profile.lock().gpu_selection.clone();

    Ok(GpuListInfo { devices, selection })
}

/// Sauvegarder la sélection GPU (appliquée au prochain démarrage)
#[tauri::command]
pub async fn set_gpu_selection(
    state: State<'_, AppState>,
    selection: GpuSelection,
) -> Result<(), String> {
    let mut profile = state.profile.lock();
    profile.gpu_selection = selection;
    profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Catalogue de modèles — HuggingFace dynamique ──────────────────────────────

/// Modèle par défaut installé lors du premier lancement
const DEFAULT_MODEL_REPO: &str = "microsoft/Phi-3-mini-4k-instruct-gguf";
const DEFAULT_MODEL_FILE: &str = "Phi-3-mini-4k-instruct-q4.gguf";
const DEFAULT_MODEL_ID: &str = "phi-3-mini-q4";

/// Entrée dans le registre local des modèles installés
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct InstalledModel {
    /// Identifiant unique (slug du repo + filename)
    pub id: String,
    /// Repo HuggingFace (ex: "microsoft/Phi-3-mini-4k-instruct-gguf")
    pub repo_id: String,
    /// Nom du fichier GGUF sur disque
    pub filename: String,
    /// Nom lisible du modèle
    pub name: String,
    /// Taille en Mo
    pub size_mb: u64,
}

/// Charger le registre des modèles installés
fn load_installed_models(data_dir: &std::path::Path) -> Vec<InstalledModel> {
    let path = data_dir.join("models").join("registry.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// Sauvegarder le registre des modèles installés
fn save_installed_models(data_dir: &std::path::Path, models: &[InstalledModel]) -> Result<(), String> {
    let dir = data_dir.join("models");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("registry.json");
    std::fs::write(&path, serde_json::to_string_pretty(models).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

/// Résultat de recherche HuggingFace
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct HfSearchResult {
    /// Identifiant du repo (ex: "TheBloke/Mistral-7B-Instruct-v0.2-GGUF")
    pub repo_id: String,
    /// Nom lisible
    pub name: String,
    /// Description courte
    pub description: String,
    /// Nombre de téléchargements
    pub downloads: u64,
    /// Nombre de likes
    pub likes: u64,
    /// Tags du modèle
    pub tags: Vec<String>,
}

/// Fichier GGUF disponible dans un repo HuggingFace
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct HfGgufFile {
    /// Nom du fichier
    pub filename: String,
    /// Taille en Mo
    pub size_mb: u64,
    /// URL de téléchargement
    pub download_url: String,
    /// Indication de quantization extraite du nom (ex: "Q4_K_M", "Q5_K_S")
    pub quantization: String,
}

/// Rechercher des modèles GGUF sur HuggingFace
#[tauri::command]
pub async fn search_huggingface(query: String) -> Result<Vec<HfSearchResult>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    // L'API HuggingFace filtre par tag "gguf" pour ne retourner que des modèles compatibles
    let url = format!(
        "https://huggingface.co/api/models?search={}&filter=gguf&sort=downloads&direction=-1&limit=15",
        urlencoding::encode(&query)
    );

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erreur réseau HuggingFace : {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HuggingFace API erreur : {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let results: Vec<HfSearchResult> = body
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            let repo_id = item.get("modelId")?.as_str()?.to_string();
            let name = repo_id.split('/').last().unwrap_or(&repo_id).to_string();
            let description = item
                .get("pipeline_tag")
                .and_then(|v| v.as_str())
                .unwrap_or("text-generation")
                .to_string();
            let downloads = item.get("downloads").and_then(|v| v.as_u64()).unwrap_or(0);
            let likes = item.get("likes").and_then(|v| v.as_u64()).unwrap_or(0);
            let tags: Vec<String> = item
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|t| t.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            Some(HfSearchResult {
                repo_id,
                name,
                description,
                downloads,
                likes,
                tags,
            })
        })
        .collect();

    Ok(results)
}

/// Lister les fichiers GGUF disponibles dans un repo HuggingFace
#[tauri::command]
pub async fn get_model_gguf_files(repo_id: String) -> Result<Vec<HfGgufFile>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://huggingface.co/api/models/{}/tree/main", repo_id);

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Erreur réseau : {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Erreur API HuggingFace : {}", response.status()));
    }

    let body: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let files: Vec<HfGgufFile> = body
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|item| {
            let filename = item.get("path")?.as_str()?.to_string();
            if !filename.to_lowercase().ends_with(".gguf") {
                return None;
            }
            let size_bytes = item.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
            let size_mb = size_bytes / 1_048_576;
            let download_url = format!(
                "https://huggingface.co/{}/resolve/main/{}",
                repo_id, filename
            );

            // Extraire la quantization du nom de fichier (ex: Q4_K_M, Q5_0, etc.)
            let quantization = extract_quantization(&filename);

            Some(HfGgufFile {
                filename,
                size_mb,
                download_url,
                quantization,
            })
        })
        .collect();

    Ok(files)
}

/// Extraire l'indication de quantization d'un nom de fichier GGUF
fn extract_quantization(filename: &str) -> String {
    let upper = filename.to_uppercase();
    let patterns = [
        "IQ1_S", "IQ1_M", "IQ2_XXS", "IQ2_XS", "IQ2_S", "IQ2_M",
        "IQ3_XXS", "IQ3_XS", "IQ3_S", "IQ3_M", "IQ4_XS", "IQ4_NL",
        "Q2_K_S", "Q2_K", "Q3_K_S", "Q3_K_M", "Q3_K_L", "Q3_K",
        "Q4_K_S", "Q4_K_M", "Q4_K_L", "Q4_K", "Q4_0", "Q4_1",
        "Q5_K_S", "Q5_K_M", "Q5_K_L", "Q5_K", "Q5_0", "Q5_1",
        "Q6_K", "Q6_0", "Q8_0", "Q8_1",
        "F16", "F32", "BF16",
    ];
    for pat in patterns {
        if upper.contains(pat) {
            return pat.to_string();
        }
    }
    "?".to_string()
}

/// Télécharger et installer un modèle GGUF depuis HuggingFace
#[tauri::command]
pub async fn download_hf_model(
    window: Window,
    state: State<'_, AppState>,
    repo_id: String,
    filename: String,
    name: String,
) -> Result<(), String> {
    let models_dir = state.data_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .map_err(|e| e.to_string())?;

    // Télécharger le fichier GGUF
    download_file_with_resume(&client, &window, &download_url, &filename, &models_dir).await?;

    // Calculer la taille du fichier téléchargé
    let size_mb = models_dir
        .join(&filename)
        .metadata()
        .map(|m| m.len() / 1_048_576)
        .unwrap_or(0);

    // Générer un ID unique basé sur le repo et le fichier
    let model_id = format!(
        "{}_{}",
        repo_id.replace('/', "_"),
        filename.trim_end_matches(".gguf")
    );

    // Ajouter au registre local
    let mut installed = load_installed_models(&state.data_dir);
    // Éviter les doublons
    installed.retain(|m| m.id != model_id);
    installed.push(InstalledModel {
        id: model_id.clone(),
        repo_id: repo_id.clone(),
        filename: filename.clone(),
        name: name.clone(),
        size_mb,
    });
    save_installed_models(&state.data_dir, &installed)?;

    // Sélectionner automatiquement le nouveau modèle
    {
        let mut profile = state.profile.lock();
        profile.selected_model = model_id.clone();
        profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    }

    tracing::info!("✅ Modèle {} ({}) téléchargé et sélectionné", name, filename);
    Ok(())
}

/// Lister les modèles installés localement
#[tauri::command]
pub async fn list_installed_models(state: State<'_, AppState>) -> Result<Vec<InstalledModelEntry>, String> {
    let installed = load_installed_models(&state.data_dir);
    let models_dir = state.data_dir.join("models");
    let selected = state.profile.lock().selected_model.clone();

    let entries = installed
        .into_iter()
        .filter(|m| models_dir.join(&m.filename).exists())
        .map(|m| {
            let active = m.id == selected;
            InstalledModelEntry { model: m, active }
        })
        .collect();

    Ok(entries)
}

#[derive(serde::Serialize, Clone)]
pub struct InstalledModelEntry {
    pub model: InstalledModel,
    pub active: bool,
}

/// Supprimer un modèle installé
#[tauri::command]
pub async fn delete_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let models_dir = state.data_dir.join("models");
    let mut installed = load_installed_models(&state.data_dir);

    let model = installed.iter().find(|m| m.id == model_id).cloned();
    let Some(model) = model else {
        return Err("Modèle inconnu".to_string());
    };

    // Si c'est le modèle actif, décharger d'abord
    let selected = state.profile.lock().selected_model.clone();
    if model_id == selected {
        *state.llm.lock() = None;
        tracing::info!("Modèle {} déchargé de la mémoire", model_id);
    }

    // Supprimer le fichier GGUF
    let gguf_path = models_dir.join(&model.filename);
    if gguf_path.exists() {
        std::fs::remove_file(&gguf_path)
            .map_err(|e| format!("Impossible de supprimer le modèle : {}", e))?;
    }

    // Supprimer le fichier partiel s'il existe
    let partial = models_dir.join(format!("{}.partial", model.filename));
    if partial.exists() {
        let _ = std::fs::remove_file(&partial);
    }

    // Retirer du registre
    installed.retain(|m| m.id != model_id);
    save_installed_models(&state.data_dir, &installed)?;

    tracing::info!("✅ Modèle {} supprimé", model_id);
    Ok(())
}

/// Sélectionner un modèle déjà installé comme modèle actif
#[tauri::command]
pub async fn select_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let installed = load_installed_models(&state.data_dir);
    let models_dir = state.data_dir.join("models");

    let model = installed.iter().find(|m| m.id == model_id)
        .ok_or_else(|| "Modèle inconnu".to_string())?;

    if !models_dir.join(&model.filename).exists() {
        return Err("Ce modèle n'est pas téléchargé".to_string());
    }

    // Décharger le modèle actuel
    *state.llm.lock() = None;

    // Mettre à jour le profil
    {
        let mut profile = state.profile.lock();
        profile.selected_model = model_id.clone();
        profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    }

    tracing::info!("Modèle {} sélectionné (redémarrage nécessaire)", model_id);
    Ok(())
}

/// Résoudre le nom de fichier GGUF du modèle sélectionné
pub fn resolve_model_filename(data_dir: &std::path::Path, selected_model: &str) -> String {
    let installed = load_installed_models(data_dir);
    installed
        .iter()
        .find(|m| m.id == selected_model)
        .map(|m| m.filename.clone())
        .unwrap_or_else(|| {
            // Fallback : ancien format de catalogue
            match selected_model {
                "phi-3-mini-q4" => "Phi-3-mini-4k-instruct-q4.gguf".to_string(),
                "phi-3.5-mini-q4" => "Phi-3.5-mini-instruct-Q4_K_M.gguf".to_string(),
                "phi-3-medium-q4" => "Phi-3-medium-4k-instruct-Q4_K_M.gguf".to_string(),
                _ => format!("{}.gguf", selected_model),
            }
        })
}

/// Télécharger un fichier avec reprise HTTP Range
async fn download_file_with_resume(
    client: &Client,
    window: &Window,
    url: &str,
    filename: &str,
    models_dir: &std::path::Path,
) -> Result<(), String> {
    let file_path = models_dir.join(filename);

    if file_path.exists() {
        tracing::info!("{} déjà présent, skip", filename);
        return Ok(());
    }

    let partial_path = models_dir.join(format!("{}.partial", filename));
    let already_downloaded = if partial_path.exists() {
        std::fs::metadata(&partial_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };

    tracing::info!("Téléchargement {} — reprise depuis {} Mo", filename, already_downloaded / 1_048_576);

    let mut request = client.get(url);
    if already_downloaded > 0 {
        request = request.header("Range", format!("bytes={}-", already_downloaded));
    }

    let response = request.send().await.map_err(|e| format!("Erreur réseau : {}", e))?;
    let total_size = response.content_length().map(|l| l + already_downloaded).unwrap_or(0);

    let mut file = if already_downloaded > 0 {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&partial_path)
            .map_err(|e| e.to_string())?;
        f.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        f
    } else {
        std::fs::File::create(&partial_path).map_err(|e| e.to_string())?
    };

    let mut downloaded = already_downloaded;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Erreur download : {}", e))?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;

        let percent = if total_size > 0 { (downloaded * 100 / total_size) as u32 } else { 0 };
        let _ = window.emit("download-progress", DownloadProgress {
            filename: filename.to_string(),
            downloaded_mb: downloaded / 1_048_576,
            total_mb: total_size / 1_048_576,
            percent,
        });
    }

    std::fs::rename(&partial_path, &file_path).map_err(|e| e.to_string())?;
    tracing::info!("✅ {} téléchargé et validé", filename);
    Ok(())
}
