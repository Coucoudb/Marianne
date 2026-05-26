// src-tauri/src/lib.rs
// Thin Tauri layer — re-exports core modules so commands can use `crate::xxx`.
pub mod commands;

// Re-export core modules (commands use `crate::xxx` paths that resolve through here)
pub use marianne_core::corpus;
pub use marianne_core::documents;
pub use marianne_core::history;
pub use marianne_core::llm;
pub use marianne_core::models;
pub use marianne_core::network;
pub use marianne_core::profile;
pub use marianne_core::prompts;
pub use marianne_core::rag;
pub use marianne_core::state;
pub use marianne_core::web;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use marianne_core::corpus;
    use marianne_core::state::AppState;
    use tauri::Manager;

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
            commands::setup::get_device_preference,
            commands::setup::set_device_preference,
            commands::setup::list_gpu_devices,
            commands::setup::set_gpu_selection,
            commands::setup::search_huggingface,
            commands::setup::get_model_gguf_files,
            commands::setup::download_hf_model,
            commands::setup::list_installed_models,
            commands::setup::delete_model,
            commands::setup::select_model,
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
