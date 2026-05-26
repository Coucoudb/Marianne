// src-tauri/src/commands/corpus.rs
use crate::corpus::updater::{CorpusUpdater, needs_update, save_last_update_timestamp};
use crate::state::AppState;
use tauri::{Emitter, State, Window};

#[derive(serde::Serialize, Clone)]
pub struct CorpusUpdateStatus {
    pub status: String,
    pub updated: usize,
    pub unchanged: usize,
    pub failed: usize,
}

/// Déclencher manuellement la mise à jour du corpus légal
#[tauri::command]
pub async fn update_corpus(
    window: Window,
    state: State<'_, AppState>,
) -> Result<CorpusUpdateStatus, String> {
    let _ = window.emit(
        "corpus-update-status",
        CorpusUpdateStatus {
            status: "started".to_string(),
            updated: 0,
            unchanged: 0,
            failed: 0,
        },
    );

    // Vérifier la connectivité avant de lancer
    let online = state.connectivity.get_or_check().await;
    if !online {
        return Err("Impossible de mettre à jour le corpus : pas de connexion internet.".to_string());
    }

    let updater = CorpusUpdater::new(state.vector_store.clone(), &state.data_dir);

    let report = updater
        .run_update()
        .await
        .map_err(|e| format!("Erreur mise à jour corpus : {}", e))?;

    save_last_update_timestamp(&state.data_dir);

    let result = CorpusUpdateStatus {
        status: "done".to_string(),
        updated: report.updated,
        unchanged: report.unchanged,
        failed: report.failed,
    };

    let _ = window.emit("corpus-update-status", result.clone());
    Ok(result)
}

/// Vérifier si une mise à jour du corpus est disponible (> 7 jours)
#[tauri::command]
pub async fn check_corpus_update(
    state: State<'_, AppState>,
) -> Result<bool, String> {
    Ok(needs_update(&state.data_dir))
}
