// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod llm;
mod rag;
mod prompts;
mod history;
mod state;

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

            tracing::info!("🇫🇷 Marianne démarre — données dans : {:?}", data_dir);

            app.manage(AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::setup::check_model_status,
            commands::setup::download_model,
            commands::setup::load_model,
            commands::setup::initialize_rag,
            commands::setup::get_device_info,
            commands::chat::send_message,
            commands::chat::get_conversation_history,
        ])
        .run(tauri::generate_context!())
        .expect("Erreur lors du lancement de Marianne");
}
