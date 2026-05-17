// src-tauri/src/commands/setup.rs
use crate::profile::DevicePreference;
use crate::state::AppState;
use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Sha256, Digest};
use std::io::{Read, Seek, SeekFrom, Write};
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
    let catalog = get_model_catalog_list();
    let model_filename = catalog.iter()
        .find(|m| m.id == selected_model)
        .map(|m| m.gguf_filename.clone())
        .unwrap_or_else(|| "phi-3-mini-q4.gguf".to_string());

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

/// Télécharger le modèle avec reprise automatique via HTTP Range
#[tauri::command]
pub async fn download_model(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let models_dir = state.data_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let downloads = vec![
        (
            "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf",
            "phi-3-mini-q4.gguf",
            // SHA256 of the GGUF file from HuggingFace — update when model changes
            Option::<&str>::None, // TODO: set expected hash once known, e.g. Some("abc123...")
        ),
    ];

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .map_err(|e| e.to_string())?;

    for (url, filename, expected_sha256) in &downloads {
        let file_path = models_dir.join(filename);

        if file_path.exists() {
            tracing::info!("{} déjà présent, skip", filename);
            continue;
        }

        let partial_path = models_dir.join(format!("{}.partial", filename));
        let already_downloaded = if partial_path.exists() {
            std::fs::metadata(&partial_path)
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        tracing::info!(
            "Téléchargement {} — reprise depuis {} Mo",
            filename,
            already_downloaded / 1_048_576
        );

        let mut request = client.get(*url);
        if already_downloaded > 0 {
            request = request.header("Range", format!("bytes={}-", already_downloaded));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Erreur réseau : {}", e))?;

        let total_size = response
            .content_length()
            .map(|l| l + already_downloaded)
            .unwrap_or(0);

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

            let percent = if total_size > 0 {
                (downloaded * 100 / total_size) as u32
            } else {
                0
            };

            let _ = window.emit(
                "download-progress",
                DownloadProgress {
                    filename: filename.to_string(),
                    downloaded_mb: downloaded / 1_048_576,
                    total_mb: total_size / 1_048_576,
                    percent,
                },
            );
        }

        std::fs::rename(&partial_path, &file_path).map_err(|e| e.to_string())?;

        // Verify file integrity via SHA256 if expected hash is provided
        if let Some(expected_hash) = expected_sha256 {
            let mut hasher = Sha256::new();
            let mut f = std::fs::File::open(&file_path).map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; 1_048_576]; // 1MB buffer
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
            let computed = format!("{:x}", hasher.finalize());
            if computed != *expected_hash {
                std::fs::remove_file(&file_path).ok();
                return Err(format!(
                    "Vérification d'intégrité échouée pour {}. Hash attendu: {}, obtenu: {}",
                    filename, expected_hash, computed
                ));
            }
            tracing::info!("✅ {} téléchargé et vérifié (SHA256 OK)", filename);
        } else {
            tracing::warn!("⚠️ {} téléchargé sans vérification de hash", filename);
        }
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
    let selected_model = profile.selected_model.clone();

    // Résoudre le nom de fichier GGUF à partir du catalogue
    let catalog = get_model_catalog_list();
    let model_filename = catalog.iter()
        .find(|m| m.id == selected_model)
        .map(|m| m.gguf_filename.clone())
        .unwrap_or_else(|| "phi-3-mini-q4.gguf".to_string());

    let engine = tokio::task::spawn_blocking(move || {
        crate::llm::engine::LlmEngine::load(&models_dir, &device_preference, &model_filename)
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

// ─── Catalogue de modèles ──────────────────────────────────────────────────────

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub size_mb: u64,
    pub gguf_url: String,
    pub gguf_filename: String,
    pub tokenizer_url: String,
    pub context_length: usize,
    pub parameters: String,
}

/// Catalogue des modèles disponibles au téléchargement
fn get_model_catalog_list() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "phi-3-mini-q4".into(),
            name: "Phi-3 Mini (Q4)".into(),
            description: "Modèle léger et rapide, idéal pour la plupart des usages. 3.8B paramètres.".into(),
            size_mb: 2300,
            gguf_url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf".into(),
            gguf_filename: "phi-3-mini-q4.gguf".into(),
            tokenizer_url: "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct/resolve/main/tokenizer.json".into(),
            context_length: 4096,
            parameters: "3.8B".into(),
        },
        ModelInfo {
            id: "phi-3.5-mini-q4".into(),
            name: "Phi-3.5 Mini (Q4)".into(),
            description: "Version améliorée de Phi-3, meilleure compréhension du français. 3.8B paramètres.".into(),
            size_mb: 2400,
            gguf_url: "https://huggingface.co/bartowski/Phi-3.5-mini-instruct-GGUF/resolve/main/Phi-3.5-mini-instruct-Q4_K_M.gguf".into(),
            gguf_filename: "phi-3.5-mini-q4.gguf".into(),
            tokenizer_url: "https://huggingface.co/microsoft/Phi-3.5-mini-instruct/resolve/main/tokenizer.json".into(),
            context_length: 4096,
            parameters: "3.8B".into(),
        },
        ModelInfo {
            id: "phi-3-medium-q4".into(),
            name: "Phi-3 Medium (Q4)".into(),
            description: "Modèle plus puissant, meilleure qualité de réponse. Nécessite plus de RAM/VRAM. 14B paramètres.".into(),
            size_mb: 8100,
            gguf_url: "https://huggingface.co/bartowski/Phi-3-medium-4k-instruct-GGUF/resolve/main/Phi-3-medium-4k-instruct-Q4_K_M.gguf".into(),
            gguf_filename: "phi-3-medium-q4.gguf".into(),
            tokenizer_url: "https://huggingface.co/microsoft/Phi-3-medium-4k-instruct/resolve/main/tokenizer.json".into(),
            context_length: 4096,
            parameters: "14B".into(),
        },
    ]
}

/// Retourner la liste des modèles disponibles avec leur statut
#[tauri::command]
pub async fn get_model_catalog(state: State<'_, AppState>) -> Result<Vec<ModelCatalogEntry>, String> {
    let models_dir = state.data_dir.join("models");
    let selected = state.profile.lock().selected_model.clone();
    let catalog = get_model_catalog_list();

    let entries = catalog.into_iter().map(|m| {
        let is_downloaded = models_dir.join(&m.gguf_filename).exists();
        let is_active = m.id == selected && is_downloaded;
        ModelCatalogEntry {
            info: m,
            downloaded: is_downloaded,
            active: is_active,
        }
    }).collect();

    Ok(entries)
}

#[derive(serde::Serialize, Clone)]
pub struct ModelCatalogEntry {
    pub info: ModelInfo,
    pub downloaded: bool,
    pub active: bool,
}

/// Supprimer le modèle actuellement chargé (libère l'espace disque)
#[tauri::command]
pub async fn delete_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let catalog = get_model_catalog_list();
    let model = catalog.iter().find(|m| m.id == model_id)
        .ok_or_else(|| "Modèle inconnu".to_string())?;

    let models_dir = state.data_dir.join("models");
    let gguf_path = models_dir.join(&model.gguf_filename);

    // Si c'est le modèle actif, décharger d'abord
    let selected = state.profile.lock().selected_model.clone();
    if model_id == selected {
        *state.llm.lock() = None;
        tracing::info!("Modèle {} déchargé de la mémoire", model_id);
    }

    // Supprimer le fichier GGUF
    if gguf_path.exists() {
        std::fs::remove_file(&gguf_path)
            .map_err(|e| format!("Impossible de supprimer le modèle : {}", e))?;
        tracing::info!("✅ Modèle {} supprimé ({:?})", model_id, gguf_path);
    }

    // Supprimer le fichier partiel s'il existe
    let partial = models_dir.join(format!("{}.partial", model.gguf_filename));
    if partial.exists() {
        let _ = std::fs::remove_file(&partial);
    }

    Ok(())
}

/// Télécharger un modèle spécifique du catalogue
#[tauri::command]
pub async fn download_selected_model(
    window: Window,
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let catalog = get_model_catalog_list();
    let model = catalog.into_iter().find(|m| m.id == model_id)
        .ok_or_else(|| "Modèle inconnu".to_string())?;

    let models_dir = state.data_dir.join("models");
    std::fs::create_dir_all(&models_dir).map_err(|e| e.to_string())?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .map_err(|e| e.to_string())?;

    // Télécharger le GGUF
    download_file_with_resume(
        &client, &window, &model.gguf_url, &model.gguf_filename, &models_dir
    ).await?;

    // Télécharger le tokenizer (partagé ou spécifique)
    let tokenizer_filename = format!("tokenizer_{}.json", model.id);
    let tokenizer_path = models_dir.join(&tokenizer_filename);
    let generic_tokenizer = models_dir.join("tokenizer.json");

    // Si pas encore de tokenizer pour ce modèle, le télécharger
    if !tokenizer_path.exists() && !generic_tokenizer.exists() {
        download_file_with_resume(
            &client, &window, &model.tokenizer_url, "tokenizer.json", &models_dir
        ).await?;
    }

    // Sauvegarder le modèle sélectionné dans le profil
    {
        let mut profile = state.profile.lock();
        profile.selected_model = model.id.clone();
        profile.save(&state.data_dir).map_err(|e| e.to_string())?;
    }

    tracing::info!("✅ Modèle {} téléchargé et sélectionné", model.id);
    Ok(())
}

/// Sélectionner un modèle déjà téléchargé (sans re-télécharger)
#[tauri::command]
pub async fn select_model(
    state: State<'_, AppState>,
    model_id: String,
) -> Result<(), String> {
    let catalog = get_model_catalog_list();
    let model = catalog.iter().find(|m| m.id == model_id)
        .ok_or_else(|| "Modèle inconnu".to_string())?;

    let models_dir = state.data_dir.join("models");
    if !models_dir.join(&model.gguf_filename).exists() {
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
