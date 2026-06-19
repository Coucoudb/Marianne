// marianne-core/src/setup.rs
// Logique d'installation et configuration automatique du modèle LLM.
// Partagée entre marianne-server et src-tauri.

use crate::llm::engine::LlmEngine;
use crate::models::{InstalledModel, load_installed_models, save_installed_models};
use crate::state::AppState;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

/// Modèle par défaut installé lors du premier lancement
pub const DEFAULT_MODEL_REPO: &str = "microsoft/Phi-3-mini-4k-instruct-gguf";
pub const DEFAULT_MODEL_FILE: &str = "Phi-3-mini-4k-instruct-q4.gguf";
pub const DEFAULT_MODEL_ID: &str = "phi-3-mini-q4";

/// Progression du téléchargement
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded_mb: u64,
    pub total_mb: u64,
    pub percent: u32,
}

/// S'assurer que le modèle par défaut est téléchargé, chargé et que le RAG est initialisé.
/// Appelé au démarrage du serveur pour garantir que Marianne est opérationnelle.
pub async fn ensure_model_ready(state: &AppState) -> Result<()> {
    // 1. Vérifier si le modèle par défaut est téléchargé
    let model_path = state.data_dir.join("models").join(DEFAULT_MODEL_FILE);
    
    if !model_path.exists() {
        tracing::info!("📥 Premier lancement : téléchargement de Phi-3 Mini (~2.2 Go)...");
        download_default_model(&state.data_dir).await
            .context("Échec du téléchargement du modèle par défaut")?;
    } else {
        tracing::info!("✅ Modèle par défaut déjà téléchargé");
    }
    
    // 2. Charger le modèle si pas déjà chargé
    if !state.is_model_loaded() {
        tracing::info!("🔄 Chargement du modèle en mémoire...");
        load_model_into_memory(state).await
            .context("Échec du chargement du modèle en mémoire")?;
    } else {
        tracing::info!("✅ Modèle déjà chargé en mémoire");
    }
    
    // 3. Initialiser le RAG si nécessaire
    let corpus_dir = state.data_dir.join("corpus");
    let needs_rag_init = !corpus_dir.exists() || {
        std::fs::read_dir(&corpus_dir)
            .ok()
            .map(|entries| {
                !entries
                    .filter_map(|e| e.ok())
                    .any(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            == Some("md")
                    })
            })
            .unwrap_or(true)
    };
    
    if needs_rag_init {
        tracing::info!("📚 Indexation du corpus légal...");
        initialize_rag_from_corpus(state).await
            .context("Échec de l'initialisation du RAG")?;
    } else {
        tracing::info!("✅ Corpus déjà présent");
    }
    
    tracing::info!("✅ Marianne prête !");
    Ok(())
}

/// Télécharger le modèle par défaut depuis HuggingFace
pub async fn download_default_model(data_dir: &Path) -> Result<()> {
    let models_dir = data_dir.join("models");
    std::fs::create_dir_all(&models_dir)
        .context("Impossible de créer le répertoire models")?;

    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        DEFAULT_MODEL_REPO, DEFAULT_MODEL_FILE
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .context("Impossible de créer le client HTTP")?;

    download_file_with_resume(
        &client, 
        &download_url, 
        DEFAULT_MODEL_FILE, 
        &models_dir,
        |_progress| {}
    ).await?;

    // Enregistrer dans le registre local
    let size_mb = models_dir
        .join(DEFAULT_MODEL_FILE)
        .metadata()
        .map(|m| m.len() / 1_048_576)
        .unwrap_or(0);

    let mut installed = load_installed_models(data_dir);
    installed.retain(|m| m.id != DEFAULT_MODEL_ID);
    installed.push(InstalledModel {
        id: DEFAULT_MODEL_ID.to_string(),
        repo_id: DEFAULT_MODEL_REPO.to_string(),
        filename: DEFAULT_MODEL_FILE.to_string(),
        name: "Phi-3 Mini (Q4)".to_string(),
        size_mb,
    });
    save_installed_models(data_dir, &installed)?;

    tracing::info!("✅ {} téléchargé et enregistré", DEFAULT_MODEL_FILE);
    Ok(())
}

/// Charger le modèle en mémoire
pub async fn load_model_into_memory(state: &AppState) -> Result<()> {
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
    let model_filename = crate::models::resolve_model_filename(&state.data_dir, &selected_model);

    let engine = tokio::task::spawn_blocking(move || {
        LlmEngine::load(&models_dir, &device_preference, &gpu_selection, &model_filename)
    })
    .await
    .context("Impossible de joindre la tâche de chargement")?
    .context("Échec du chargement du modèle")?;

    *state.llm.lock() = Some(engine);

    tracing::info!("✅ Modèle {} chargé en mémoire", selected_model);
    Ok(())
}

/// Initialiser le RAG (ingestion du corpus légal)
pub async fn initialize_rag_from_corpus(state: &AppState) -> Result<()> {
    let corpus_dir = state.data_dir.join("corpus");
    let models_dir = state.data_dir.join("models");

    if !corpus_dir.exists() {
        tracing::info!("Création du répertoire corpus : {:?}", corpus_dir);
        std::fs::create_dir_all(&corpus_dir)
            .context("Impossible de créer le répertoire corpus")?;
    }

    // Seed : copier les fiches bundlées si le corpus est vide (uniquement pour Tauri)
    seed_corpus_from_embedded(&corpus_dir);

    let store = state.vector_store.clone();
    let chunks = crate::rag::ingestion::ingest_corpus(&corpus_dir, &store, &models_dir)
        .await
        .context("Échec de l'ingestion du corpus")?;

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

    tracing::info!("✅ RAG initialisé : {} chunks", chunks);
    Ok(())
}

/// Copier les fiches Markdown embarquées dans le corpus si celui-ci est vide.
/// Cette fonction est un no-op dans marianne-server (pas de ressources bundlées).
/// Elle peut être réimplémentée dans src-tauri avec le code d'accès aux ressources.
fn seed_corpus_from_embedded(corpus_dir: &Path) {
    // Vérifier si le corpus contient déjà des .md
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

    // Dans marianne-server, copier depuis le répertoire corpus/ du projet
    let project_corpus = std::env::current_dir()
        .ok()
        .and_then(|p| {
            // Tenter de trouver le corpus dans marianne/corpus/
            let candidate = p.join("corpus");
            if candidate.exists() {
                Some(candidate)
            } else {
                // Tenter un niveau au-dessus
                p.parent()
                    .map(|parent| parent.join("corpus"))
                    .filter(|c| c.exists())
            }
        });

    if let Some(source_corpus) = project_corpus {
        let mut copied = 0usize;
        if let Ok(entries) = std::fs::read_dir(&source_corpus) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    let dest = corpus_dir.join(path.file_name().unwrap());
                    if !dest.exists()
                        && std::fs::copy(&path, &dest).is_ok() {
                            copied += 1;
                        }
                }
            }
        }

        if copied > 0 {
            tracing::info!("📚 {} fiches initiales copiées dans le corpus", copied);
        }
    }
}

/// Télécharger un fichier avec reprise HTTP Range.
/// Le callback `on_progress` est appelé à chaque chunk téléchargé.
async fn download_file_with_resume<F>(
    client: &Client,
    url: &str,
    filename: &str,
    models_dir: &Path,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(DownloadProgress),
{
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

    let response = request.send().await.context("Erreur réseau lors du téléchargement")?;
    let total_size = response.content_length().map(|l| l + already_downloaded).unwrap_or(0);

    let mut file = if already_downloaded > 0 {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&partial_path)
            .context("Impossible d'ouvrir le fichier partiel")?;
        f.seek(SeekFrom::End(0)).context("Impossible de seek dans le fichier")?;
        f
    } else {
        std::fs::File::create(&partial_path).context("Impossible de créer le fichier partiel")?
    };

    let mut downloaded = already_downloaded;
    let mut stream = response.bytes_stream();

    let pb = indicatif::ProgressBar::new(total_size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} ({eta})")
            .unwrap()
            .progress_chars("#>-")
    );
    pb.set_message(filename.to_string());
    pb.set_position(downloaded);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Erreur lors du téléchargement d'un chunk")?;
        file.write_all(&chunk).context("Impossible d'écrire dans le fichier")?;
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);

        let percent = downloaded
            .checked_mul(100)
            .and_then(|v| v.checked_div(total_size))
            .unwrap_or(0) as u32;
        on_progress(DownloadProgress {
            filename: filename.to_string(),
            downloaded_mb: downloaded / 1_048_576,
            total_mb: total_size / 1_048_576,
            percent,
        });
    }

    pb.finish_and_clear();

    std::fs::rename(&partial_path, &file_path).context("Impossible de renommer le fichier final")?;
    tracing::info!("✅ {} téléchargé et validé", filename);
    Ok(())
}

/// Télécharger un modèle depuis HuggingFace avec callback de progression
pub async fn download_model_from_huggingface<F>(
    data_dir: &Path,
    repo_id: &str,
    filename: &str,
    name: &str,
    on_progress: F,
) -> Result<String>
where
    F: FnMut(DownloadProgress),
{
    let models_dir = data_dir.join("models");
    std::fs::create_dir_all(&models_dir).context("Impossible de créer le répertoire models")?;

    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(7200))
        .build()
        .context("Impossible de créer le client HTTP")?;

    download_file_with_resume(&client, &download_url, filename, &models_dir, on_progress).await?;

    // Calculer la taille du fichier téléchargé
    let size_mb = models_dir
        .join(filename)
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
    let mut installed = load_installed_models(data_dir);
    installed.retain(|m| m.id != model_id);
    installed.push(InstalledModel {
        id: model_id.clone(),
        repo_id: repo_id.to_string(),
        filename: filename.to_string(),
        name: name.to_string(),
        size_mb,
    });
    save_installed_models(data_dir, &installed)?;

    tracing::info!("✅ Modèle {} ({}) téléchargé et enregistré", name, filename);
    Ok(model_id)
}
