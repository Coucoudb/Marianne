// src-tauri/src/commands/chat.rs
// Thin Tauri IPC layer — delegates all logic to marianne_core::chat.
// The only Tauri-specific code here is translating ChatEvent -> window.emit().

use crate::state::AppState;
use marianne_core::chat::{process_chat, ChatEvent, ChatRequest};
use marianne_core::prompts::system::ConversationTurn;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{Emitter, State, Window};

/// Envoyer un message et recevoir la reponse en streaming via Tauri events.
#[tauri::command]
pub async fn send_message(
    window: Window,
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<String, String> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ChatEvent>(64);

    // Spawn the core pipeline
    let state_clone = state.inner().clone();
    let core_handle = tokio::spawn(async move {
        process_chat(state_clone, request, tx).await
    });

    // Relay events to the Tauri WebView
    while let Some(event) = rx.recv().await {
        match &event {
            ChatEvent::ContradictionWarning { conversation_id, message } => {
                #[derive(Serialize, Clone)]
                struct ContradictionWarning { conversation_id: String, message: String }
                let _ = window.emit("contradiction-warning", ContradictionWarning {
                    conversation_id: conversation_id.clone(),
                    message: message.clone(),
                });
            }
            ChatEvent::ConfidenceInfo { conversation_id, score, reason, web_search_triggered } => {
                #[derive(Serialize, Clone)]
                struct ConfidenceInfo {
                    conversation_id: String,
                    score: f32,
                    reason: String,
                    web_search_triggered: bool,
                }
                let _ = window.emit("confidence-info", ConfidenceInfo {
                    conversation_id: conversation_id.clone(),
                    score: *score,
                    reason: reason.clone(),
                    web_search_triggered: *web_search_triggered,
                });
            }
            ChatEvent::OfflineMode { confidence, message } => {
                #[derive(Serialize, Clone)]
                struct OfflineNotice { confidence: f32, message: String }
                let _ = window.emit("offline-mode", OfflineNotice {
                    confidence: *confidence,
                    message: message.clone(),
                });
            }
            ChatEvent::WebSearchStatus { conversation_id, status, sources_count } => {
                #[derive(Serialize, Clone)]
                struct WebSearchStatus {
                    conversation_id: String,
                    status: String,
                    sources_count: usize,
                }
                let _ = window.emit("web-search-status", WebSearchStatus {
                    conversation_id: conversation_id.clone(),
                    status: status.clone(),
                    sources_count: *sources_count,
                });
            }
            ChatEvent::StreamToken { token, conversation_id } => {
                #[derive(Serialize, Clone)]
                struct StreamToken { token: String, conversation_id: String }
                let _ = window.emit("stream-token", StreamToken {
                    token: token.clone(),
                    conversation_id: conversation_id.clone(),
                });
            }
            ChatEvent::GenerationDone {
                conversation_id,
                full_response,
                sources,
                tokens_generated,
                time_ms,
            } => {
                #[derive(Serialize, Clone)]
                struct GenerationDone {
                    conversation_id: String,
                    full_response: String,
                    sources: Vec<String>,
                    tokens_generated: usize,
                    time_ms: u64,
                }
                let _ = window.emit("generation-done", GenerationDone {
                    conversation_id: conversation_id.clone(),
                    full_response: full_response.clone(),
                    sources: sources.clone(),
                    tokens_generated: *tokens_generated,
                    time_ms: *time_ms,
                });
            }
        }
    }

    core_handle
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Arreter la generation en cours.
#[tauri::command]
pub async fn stop_generation(state: State<'_, AppState>) -> Result<(), String> {
    state.abort_generation.store(true, Ordering::SeqCst);
    tracing::info!("Arret de generation demande par l utilisateur");
    Ok(())
}

/// Recuperer l historique d une conversation.
#[tauri::command]
pub async fn get_conversation_history(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ConversationTurn>, String> {
    state
        .history
        .get_conversation(&conversation_id)
        .await
        .map_err(|e| e.to_string())
}
