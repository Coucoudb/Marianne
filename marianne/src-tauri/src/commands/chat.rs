// src-tauri/src/commands/chat.rs
use crate::llm::confidence::{detect_category, evaluate_rag_confidence, is_conversational, is_off_topic, OFF_TOPIC_RESPONSE};
use crate::llm::streamer::BatchStreamer;
use crate::prompts::system::{build_prompt, ConversationTurn};
use crate::rag::feedback::ingest_web_results;
use crate::rag::retriever::Retriever;
use crate::state::AppState;
use crate::web::cache::WebCache;
use crate::web::searcher::{WebResult, WebSearcher};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{Emitter, State, Window};

#[derive(Serialize, Clone)]
pub struct OfflineNotice {
    pub confidence: f32,
    pub message: String,
}

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

#[derive(Serialize, Clone)]
pub struct WebSearchStatus {
    pub conversation_id: String,
    pub status: String,
    pub sources_count: usize,
}

#[derive(Serialize, Clone)]
pub struct ConfidenceInfo {
    pub conversation_id: String,
    pub score: f32,
    pub reason: String,
    pub web_search_triggered: bool,
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
    let max_tokens = request.max_tokens.unwrap_or(2048);

    if !state.is_model_loaded() {
        return Err("Le modèle n'est pas encore chargé. Veuillez attendre.".to_string());
    }

    // 0. Filtre hors sujet — bloque avant RAG et LLM
    if is_off_topic(&request.message) {
        tracing::info!("Question hors sujet bloquée : '{}'", &request.message[..50.min(request.message.len())]);
        let _ = window.emit(
            "stream-token",
            StreamToken {
                token: OFF_TOPIC_RESPONSE.to_string(),
                conversation_id: conv_id.clone(),
            },
        );
        let elapsed = start_time.elapsed().as_millis() as u64;
        let _ = window.emit(
            "generation-done",
            GenerationDone {
                conversation_id: conv_id.clone(),
                full_response: OFF_TOPIC_RESPONSE.to_string(),
                sources: Vec::new(),
                tokens_generated: 0,
                time_ms: elapsed,
            },
        );
        return Ok(conv_id);
    }

    // 1. Pipeline RAG : trouver le contexte pertinent (optionnel — peut ne pas être initialisé)
    let retriever = Retriever::new(state.vector_store.clone());
    let (rag_context, sources, rag_scores) = match retriever.retrieve(&request.message, 3).await {
        Ok(rag_results) => {
            let context = Retriever::format_context(&rag_results);
            let scores: Vec<f32> = rag_results.iter().map(|r| r.similarity).collect();
            let srcs: Vec<String> = rag_results
                .iter()
                .map(|r| r.source.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            (context, srcs, scores)
        }
        Err(e) => {
            tracing::debug!("RAG non disponible, génération sans contexte : {}", e);
            (String::new(), Vec::new(), Vec::new())
        }
    };

    // 1b. Évaluer la confiance et déclencher la recherche web si nécessaire
    let confidence = evaluate_rag_confidence(
        &rag_scores,
        rag_context.len(),
        request.message.len(),
    );

    let _ = window.emit(
        "confidence-info",
        ConfidenceInfo {
            conversation_id: conv_id.clone(),
            score: confidence.score,
            reason: confidence.reason.clone(),
            web_search_triggered: confidence.should_search_web,
        },
    );

    let (web_context, all_sources) = if confidence.should_search_web && !is_conversational(&request.message) {
        // Vérifier la connectivité AVANT le timeout réseau
        let online = state.connectivity.get_or_check().await;

        if !online {
            tracing::info!("Hors-ligne détecté — skip recherche web");
            let _ = window.emit(
                "offline-mode",
                OfflineNotice {
                    confidence: confidence.score,
                    message: "Mode hors-ligne : Marianne répond depuis sa base locale.".to_string(),
                },
            );
            (String::new(), sources)
        } else {
        let _ = window.emit(
            "web-search-status",
            WebSearchStatus {
                conversation_id: conv_id.clone(),
                status: "started".to_string(),
                sources_count: 0,
            },
        );

        let category = detect_category(&request.message);
        let cache = WebCache::new(&state.data_dir.join("web_cache"));
        
        let web_results = if let Some(cached) = cache.get(&request.message, category) {
            tracing::info!("Cache web hit pour '{}'", &request.message[..30.min(request.message.len())]);
            cached
        } else {
            match WebSearcher::new() {
                Ok(searcher) => {
                    match searcher.search(&request.message, category, 3).await {
                        Ok(results) => {
                            cache.set(&request.message, category, &results).ok();
                            results
                        }
                        Err(e) => {
                            tracing::warn!("Recherche web échouée : {}", e);
                            Vec::new()
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Impossible de créer le client web : {}", e);
                    Vec::new()
                }
            }
        };

        let web_ctx = format_web_context(&web_results);
        let web_srcs: Vec<String> = web_results.iter()
            .filter(|r| is_web_result_relevant(r, &request.message))
            .map(|r| r.url.clone())
            .collect();

        // Feedback loop : injecter les résultats web de qualité dans le RAG
        let quality_results: Vec<WebResult> = web_results
            .iter()
            .filter(|r| r.content.len() >= 200)
            .cloned()
            .collect();
        if !quality_results.is_empty() {
            let store = state.vector_store.clone();
            let hashes = state.known_hashes.clone();
            let cat = category.to_string();
            tokio::spawn(async move {
                match ingest_web_results(&quality_results, &store, &hashes, &cat).await {
                    Ok(n) if n > 0 => tracing::info!("Feedback loop : {} chunks web → RAG", n),
                    Ok(_) => {},
                    Err(e) => tracing::warn!("Feedback loop échoué : {}", e),
                }
            });
        }

        let _ = window.emit(
            "web-search-status",
            WebSearchStatus {
                conversation_id: conv_id.clone(),
                status: "done".to_string(),
                sources_count: web_results.len(),
            },
        );

        let mut combined_sources = sources;
        combined_sources.extend(web_srcs);
        (web_ctx, combined_sources)
        } // fin du else (online)
    } else {
        (String::new(), sources)
    };

    // Combiner RAG local + contexte web
    let full_context = if web_context.is_empty() {
        rag_context
    } else if rag_context.is_empty() {
        web_context
    } else {
        format!("{}\n\n{}", rag_context, web_context)
    };

    // 2. Récupérer l'historique de conversation
    let history = state
        .history
        .get_conversation(&conv_id)
        .await
        .unwrap_or_default();

    // 3. Construire le prompt
    let profile = state.profile.lock().clone();
    let prompt = build_prompt(&request.message, &full_context, &history, &profile);
    tracing::info!("Prompt construit ({} caractères) — lancement de la génération...", prompt.len());

    // 4. Génération en streaming avec batching IPC
    let llm_state = state.llm.clone();
    let abort_flag = state.abort_generation.clone();
    abort_flag.store(false, Ordering::SeqCst);
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
            // Vérifier si l'utilisateur a demandé l'arrêt
            if abort_flag.load(Ordering::SeqCst) {
                tracing::info!("Génération interrompue par l'utilisateur");
                return false;
            }
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
            sources: all_sources,
            tokens_generated: full_response.1,
            time_ms: elapsed,
        },
    );

    state.touch_llm();
    Ok(conv_id)
}

/// Arrêter la génération en cours
#[tauri::command]
pub async fn stop_generation(
    state: State<'_, AppState>,
) -> Result<(), String> {
    state.abort_generation.store(true, Ordering::SeqCst);
    tracing::info!("Arrêt de génération demandé par l'utilisateur");
    Ok(())
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

/// Formater les résultats web en contexte pour le LLM
fn format_web_context(results: &[WebResult]) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut context = String::from("INFORMATIONS WEB OFFICIELLES (utilise ces informations pour répondre, mais ne recopie PAS les en-têtes ni les URLs ci-dessous) :\n\n");

    for (i, result) in results.iter().enumerate() {
        context.push_str(&format!(
            "Source {} — {} :\n{}\n\n",
            i + 1,
            result.source_name,
            result.content.chars().take(800).collect::<String>()
        ));
    }

    context
}
/// Vérifier qu'un résultat web a un lien sémantique minimal avec la question.
/// Évite d'afficher des sources hors sujet (ex: page Covid pour une question juridique).
fn is_web_result_relevant(result: &WebResult, query: &str) -> bool {
    let query_lower = query.to_lowercase();
    let content_lower = result.content.to_lowercase();
    let title_lower = result.title.to_lowercase();

    // Extraire les mots significatifs de la question (> 3 chars, pas des mots vides)
    let stop_words = [
        "quel", "quelle", "quels", "quelles", "est", "sont", "dans", "pour",
        "avec", "cette", "les", "des", "une", "que", "qui", "comment",
        "plus", "moins", "par", "sur", "aux", "fait", "faire", "peut",
        "entre", "comme", "mais", "aussi", "tout", "tous", "bien",
        "avoir", "r\u{00e9}cemment", "r\u{00e9}cente", "ancien", "ancienne",
    ];

    let query_words: Vec<&str> = query_lower
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .filter(|w| w.len() > 3 && !stop_words.contains(w))
        .collect();

    if query_words.is_empty() {
        return true; // Pas assez de mots pour filtrer
    }

    // Au moins 1 mot significatif de la question doit appara\u{00ee}tre dans le titre ou contenu
    let matches = query_words.iter()
        .filter(|w| content_lower.contains(*w) || title_lower.contains(*w))
        .count();

    let ratio = matches as f32 / query_words.len() as f32;
    ratio >= 0.2 // Au moins 20% des mots cl\u{00e9}s doivent matcher
}