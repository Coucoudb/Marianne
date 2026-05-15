// src-tauri/src/commands/chat.rs
use crate::llm::streamer::BatchStreamer;
use crate::prompts::system::{build_prompt, ConversationTurn};
use crate::rag::retriever::Retriever;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State, Window};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
    pub max_tokens: Option<usize>,
}

#[derive(Serialize, Clone)]
pub struct StreamToken {
    pub token: String,
    pub conversation_id: String,
}

#[derive(Serialize, Clone)]
pub struct GenerationDone {
    pub conversation_id: String,
    pub full_response: String,
    pub sources: Vec<String>,
    pub tokens_generated: usize,
    pub time_ms: u64,
}

/// Envoyer un message et recevoir la réponse en streaming
#[tauri::command]
pub async fn send_message(
    window: Window,
    state: State<'_, AppState>,
    request: ChatRequest,
) -> Result<String, String> {
    let start_time = std::time::Instant::now();
    let conv_id = request
        .conversation_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let max_tokens = request.max_tokens.unwrap_or(1024);

    if !state.is_model_loaded() {
        return Err("Le modèle n'est pas encore chargé. Veuillez attendre.".to_string());
    }

    // 1. Pipeline RAG : trouver le contexte pertinent (optionnel — peut ne pas être initialisé)
    let retriever = Retriever::new(state.vector_store.clone());
    let (rag_context, sources) = match retriever.retrieve(&request.message, 3).await {
        Ok(rag_results) => {
            let context = Retriever::format_context(&rag_results);
            let srcs: Vec<String> = rag_results
                .iter()
                .map(|r| r.source.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            (context, srcs)
        }
        Err(e) => {
            tracing::debug!("RAG non disponible, génération sans contexte : {}", e);
            (String::new(), Vec::new())
        }
    };

    // 2. Récupérer l'historique de conversation
    let history = state
        .history
        .get_conversation(&conv_id)
        .await
        .unwrap_or_default();

    // 3. Construire le prompt
    let prompt = build_prompt(&request.message, &rag_context, &history);
    tracing::info!("Prompt construit ({} caractères) — lancement de la génération...", prompt.len());

    // 4. Génération en streaming avec batching IPC
    let llm_state = state.llm.clone();
    let window_clone = window.clone();
    let conv_id_clone = conv_id.clone();

    let full_response = tokio::task::spawn_blocking(move || {
        let mut guard = llm_state.lock();
        let engine = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Moteur LLM non disponible"))?;

        tracing::info!("Début du prefill...");
        let mut streamer = BatchStreamer::new();
        let mut tokens_count = 0usize;

        let response = engine.generate_streaming(&prompt, max_tokens, |token| {
            tokens_count += 1;
            if let Some(batch) = streamer.push(token) {
                let _ = window_clone.emit(
                    "stream-token",
                    StreamToken {
                        token: batch,
                        conversation_id: conv_id_clone.clone(),
                    },
                );
            }
            true
        })?;

        // Flush les tokens restants
        if let Some(remaining) = streamer.flush() {
            let _ = window_clone.emit(
                "stream-token",
                StreamToken {
                    token: remaining,
                    conversation_id: conv_id_clone.clone(),
                },
            );
        }

        Ok::<(String, usize), anyhow::Error>((response, tokens_count))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let elapsed = start_time.elapsed().as_millis() as u64;

    // 5. Sauvegarder dans l'historique
    state
        .history
        .save_turn(
            &conv_id,
            &request.message,
            &full_response.0,
        )
        .await
        .ok();

    // 6. Émettre l'événement de fin
    let _ = window.emit(
        "generation-done",
        GenerationDone {
            conversation_id: conv_id.clone(),
            full_response: full_response.0,
            sources,
            tokens_generated: full_response.1,
            time_ms: elapsed,
        },
    );

    state.touch_llm();
    Ok(conv_id)
}

/// Récupérer l'historique d'une conversation
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
