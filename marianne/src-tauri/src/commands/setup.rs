// src-tauri/src/commands/setup.rs
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

/// Vérifier l'état du modèle et du système
#[tauri::command]
pub async fn check_model_status(state: State<'_, AppState>) -> Result<ModelStatus, String> {
    let model_path = state.data_dir.join("models/phi-3-mini-q4.gguf");
    let tokenizer_path = state.data_dir.join("models/tokenizer.json");
    let available_ram_gb = read_available_ram_gb();

    Ok(ModelStatus {
        model_downloaded: model_path.exists(),
        tokenizer_downloaded: tokenizer_path.exists(),
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
        ),
        (
            "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct/resolve/main/tokenizer.json",
            "tokenizer.json",
        ),
    ];

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .map_err(|e| e.to_string())?;

    for (url, filename) in downloads {
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

        let mut request = client.get(url);
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
        tracing::info!("✅ {} téléchargé et validé", filename);
    }

    Ok(())
}

/// Charger Phi-3-Mini en mémoire
#[tauri::command]
pub async fn load_model(
    window: Window,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let _ = window.emit("model-loading", "Chargement de Marianne en mémoire...");

    let models_dir = state.data_dir.join("models");
    let engine = tokio::task::spawn_blocking(move || {
        crate::llm::engine::LlmEngine::load(&models_dir)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    *state.llm.lock() = Some(engine);

    let _ = window.emit("model-ready", "Marianne est prête !");
    tracing::info!("✅ Phi-3-Mini chargé en mémoire");
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
        tracing::warn!("Répertoire corpus inexistant : {:?}", corpus_dir);
        return Ok(());
    }

    let store = state.vector_store.clone();
    let chunks = crate::rag::ingestion::ingest_corpus(&corpus_dir, &store, &models_dir)
        .await
        .map_err(|e| e.to_string())?;

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
