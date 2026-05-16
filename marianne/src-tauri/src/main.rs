// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod corpus;
mod documents;
mod llm;
mod network;
mod profile;
mod rag;
mod prompts;
mod history;
mod state;
mod web;

use state::AppState;
use tauri::Manager;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("marianne=info".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("Impossible de trouver AppData");

            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(data_dir.join("models"))?;
            std::fs::create_dir_all(data_dir.join("db"))?;
            std::fs::create_dir_all(data_dir.join("graph"))?;
            std::fs::create_dir_all(data_dir.join("web_cache"))?;
            std::fs::create_dir_all(data_dir.join("corpus_hashes"))?;

            tracing::info!("🇫🇷 Marianne démarre — données dans : {:?}", data_dir);

            let state = AppState::new(data_dir.clone());
            app.manage(state);

            // Vérification hebdomadaire du corpus en arrière-plan
            if corpus::updater::needs_update(&data_dir) {
                let store = app.state::<AppState>().vector_store.clone();
                let dir = data_dir.clone();
                tauri::async_runtime::spawn(async move {
                    // Attendre 10s que l'app soit prête
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    tracing::info!("Mise à jour hebdomadaire du corpus légal...");
                    let updater = corpus::updater::CorpusUpdater::new(store, &dir);
                    if let Ok(report) = updater.run_update().await {
                        if report.updated > 0 {
                            corpus::updater::save_last_update_timestamp(&dir);
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::setup::check_model_status,
            commands::setup::download_model,
            commands::setup::load_model,
            commands::setup::initialize_rag,
            commands::setup::get_device_info,
            commands::chat::send_message,
            commands::chat::stop_generation,
            commands::chat::get_conversation_history,
            commands::profile::get_profile,
            commands::profile::save_profile,
            commands::documents::extract_document,
            commands::corpus::update_corpus,
            commands::corpus::check_corpus_update,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de Marianne");
}
