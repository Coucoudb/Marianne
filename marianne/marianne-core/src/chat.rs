// marianne-core/src/chat.rs
// Pipeline de chat générique — sans dépendance Tauri ni HTTP.
// Le transport (IPC Tauri, SSE Axum, …) est découplé via un Sender<ChatEvent>.

use crate::llm::confidence::{detect_category, evaluate_rag_confidence, is_conversational, is_off_topic, requires_web_search, OFF_TOPIC_RESPONSE};
use crate::llm::streamer::BatchStreamer;
use crate::prompts::system::{build_deep_think_prompt, build_prompt};
use crate::rag::feedback::ingest_web_results;
use crate::rag::retriever::Retriever;
use crate::state::AppState;
use crate::web::cache::WebCache;
use crate::web::searcher::{WebResult, WebSearcher};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tokio::sync::mpsc;

// ─── Types publics ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
    pub max_tokens: Option<usize>,
    pub agent_id: Option<String>,
    pub deep_think: Option<bool>,
}

/// Tous les événements émis pendant le pipeline de chat.
/// Le consommateur (Tauri, SSE Axum, …) mappe ces variants à son protocole.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ChatEvent {
    ContradictionWarning {
        conversation_id: String,
        message: String,
    },
    ConfidenceInfo {
        conversation_id: String,
        score: f32,
        reason: String,
        web_search_triggered: bool,
    },
    OfflineMode {
        confidence: f32,
        message: String,
    },
    WebSearchStatus {
        conversation_id: String,
        status: String,
        sources_count: usize,
    },
    DeepThinkStep {
        phase: String,
        content: String,
        conversation_id: String,
    },
    StreamToken {
        token: String,
        conversation_id: String,
    },
    GenerationDone {
        conversation_id: String,
        full_response: String,
        sources: Vec<String>,
        tokens_generated: usize,
        time_ms: u64,
    },
}

// ─── Pipeline principal ──────────────────────────────────────────────────────

/// Exécuter le pipeline de chat complet.
/// Retourne le `conversation_id` utilisé pour cette session.
/// Les événements intermédiaires sont envoyés via `tx` ; le consommateur
/// peut fermer le receiver pour interrompre la génération côté abonné.
pub async fn process_chat(
    state: AppState,
    request: ChatRequest,
    tx: mpsc::Sender<ChatEvent>,
) -> anyhow::Result<String> {
    // Security: limit input message length to prevent DoS
    if request.message.len() > 10_000 {
        anyhow::bail!("Message trop long (max 10 000 caractères).");
    }
    if request.message.trim().is_empty() {
        anyhow::bail!("Message vide.");
    }

    let start_time = std::time::Instant::now();
    let conv_id = request
        .conversation_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let max_tokens = request.max_tokens.unwrap_or(2048);

    if !state.is_model_loaded() {
        anyhow::bail!("Le modèle n'est pas encore chargé. Veuillez attendre.");
    }

    let (agent, agent_skills) = if let Some(id) = &request.agent_id {
        let all_agents = state.workspace.list_agents().await.unwrap_or_default();
        if let Some(a) = all_agents.into_iter().find(|a| &a.id == id) {
            // Câbler le project_dir du workspace pour activer le niveau Projet
            if let Some(ref wd) = a.working_directory {
                let wd_path = std::path::PathBuf::from(wd);
                if wd_path.exists() {
                    state.workspace.set_project_dir(Some(wd_path));
                }
            }
            let all_skills = state.workspace.list_skills().await.unwrap_or_default();
            let skills: Vec<crate::workspace::skill::Skill> = all_skills.into_iter().filter(|s| a.skills.contains(&s.id)).collect();
            (Some(a), skills)
        } else {
            (None, vec![])
        }
    } else {
        (None, vec![])
    };

    // 0. Filtre hors sujet — bloque avant RAG et LLM
    if is_off_topic(&request.message) {
        tracing::info!(
            "Question hors sujet bloquée : '{}'",
            &request.message[..50.min(request.message.len())]
        );
        let elapsed = start_time.elapsed().as_millis() as u64;
        let _ = tx
            .send(ChatEvent::StreamToken {
                token: OFF_TOPIC_RESPONSE.to_string(),
                conversation_id: conv_id.clone(),
            })
            .await;
        let _ = tx
            .send(ChatEvent::GenerationDone {
                conversation_id: conv_id.clone(),
                full_response: OFF_TOPIC_RESPONSE.to_string(),
                sources: Vec::new(),
                tokens_generated: 0,
                time_ms: elapsed,
            })
            .await;
        return Ok(conv_id);
    }

    // 0b. Messages conversationnels — skip RAG et recherche web
    let is_conv = is_conversational(&request.message);

    // 1. Détection de catégorie
    let category = detect_category(&request.message);

    // 1b. Pipeline parallèle : RAG + historique
    let (rag_results, history) = if is_conv {
        let hist = state
            .history
            .get_conversation(&conv_id)
            .await
            .unwrap_or_default();
        (Vec::new(), hist)
    } else {
        let store_clone = state.vector_store.clone();
        let graph_clone = state.knowledge_graph.clone();
        let msg_clone = request.message.clone();
        let cat_clone = category.to_string();
        let history_db = state.history.clone();
        let conv_id_hist = conv_id.clone();

        let (rag_res, hist_res) = tokio::join!(
            async {
                let r = Retriever::new(store_clone, graph_clone);
                r.retrieve(&msg_clone, 5, Some(&cat_clone))
                    .await
                    .unwrap_or_default()
            },
            async {
                history_db
                    .get_conversation(&conv_id_hist)
                    .await
                    .unwrap_or_default()
            }
        );
        (rag_res, hist_res)
    };

    let rag_context = if rag_results.is_empty() {
        String::new()
    } else {
        Retriever::format_context(&rag_results)
    };

    let rag_scores: Vec<f32> = rag_results.iter().map(|r| r.score).collect();
    let sources: Vec<String> = rag_results
        .iter()
        .map(|r| r.source.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // 1c. Contradictions web/corpus
    if !is_conv {
        if let Some(warning) = Retriever::detect_contradictions(&rag_results) {
            let _ = tx
                .send(ChatEvent::ContradictionWarning {
                    conversation_id: conv_id.clone(),
                    message: warning,
                })
                .await;
        }
    }

    // 1d. Confiance RAG
    let confidence = evaluate_rag_confidence(
        &rag_scores,
        rag_context.len(),
        request.message.len(),
        category,
    );

    if !is_conv {
        let _ = tx
            .send(ChatEvent::ConfidenceInfo {
                conversation_id: conv_id.clone(),
                score: confidence.score,
                reason: confidence.reason.clone(),
                web_search_triggered: confidence.should_search_web,
            })
            .await;
    }

    // Forcer la recherche web pour les questions temporelles/actualité
    let force_web = requires_web_search(&request.message);
    if force_web {
        tracing::info!("Question temporelle/actualité détectée — recherche web forcée");
    }

    let should_search_web = if let Some(a) = &agent {
        if let Some(w) = &a.web_search {
            w.enabled
        } else {
            confidence.should_search_web || force_web
        }
    } else {
        confidence.should_search_web || force_web
    };
    // DeepThink force la recherche web pour enrichir le contexte
    let should_search_web = should_search_web || request.deep_think == Some(true);

    // 2. Recherche web optionnelle
    let (web_context, all_sources) = if should_search_web && !is_conv {
        let online = state.connectivity.get_or_check().await;

        if !online {
            tracing::info!("Hors-ligne détecté — skip recherche web");
            let _ = tx
                .send(ChatEvent::OfflineMode {
                    confidence: confidence.score,
                    message: "Mode hors-ligne : Marianne répond depuis sa base locale.".to_string(),
                })
                .await;
            (String::new(), sources)
        } else {
            let _ = tx
                .send(ChatEvent::WebSearchStatus {
                    conversation_id: conv_id.clone(),
                    status: "started".to_string(),
                    sources_count: 0,
                })
                .await;

            let cache = WebCache::new(&state.data_dir.join("web_cache"));

            let web_results = if let Some(cached) = cache.get(&request.message, category) {
                tracing::info!(
                    "Cache web hit pour '{}'",
                    &request.message[..30.min(request.message.len())]
                );
                cached
            } else {
                match WebSearcher::new() {
                    Ok(searcher) => match searcher.search(&request.message, category, 5).await {
                        Ok(results) => {
                            cache.set(&request.message, category, &results).ok();
                            results
                        }
                        Err(e) => {
                            tracing::warn!("Recherche web échouée : {}", e);
                            Vec::new()
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Impossible de créer le client web : {}", e);
                        Vec::new()
                    }
                }
            };

            let relevant_web: Vec<WebResult> = web_results
                .into_iter()
                .filter(|r| is_web_result_relevant(r, &request.message))
                .collect();

            let web_ctx = format_web_context(&relevant_web);

            let mut seen_domains = std::collections::HashSet::new();
            let web_srcs: Vec<String> = relevant_web
                .iter()
                .filter(|r| {
                    let domain = extract_domain(&r.url);
                    seen_domains.insert(domain)
                })
                .map(|r| r.url.clone())
                .collect();

            // Feedback loop : injecter les résultats web de qualité dans le RAG
            let quality_results: Vec<WebResult> = relevant_web
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
                        Ok(_) => {}
                        Err(e) => tracing::warn!("Feedback loop échoué : {}", e),
                    }
                });
            }

            let _ = tx
                .send(ChatEvent::WebSearchStatus {
                    conversation_id: conv_id.clone(),
                    status: "done".to_string(),
                    sources_count: relevant_web.len(),
                })
                .await;

            let mut combined_sources = sources;
            combined_sources.extend(web_srcs);
            let mut final_domains = std::collections::HashSet::new();
            combined_sources.retain(|url| {
                let domain = extract_domain(url);
                final_domains.insert(domain)
            });
            (web_ctx, combined_sources)
        }
    } else {
        (String::new(), sources)
    };

    // 3. Combiner contextes + construire le prompt
    let full_context = match (rag_context.is_empty(), web_context.is_empty()) {
        (true, _) => web_context,
        (_, true) => rag_context,
        _ => format!("{}\n\n{}", rag_context, web_context),
    };

    let profile = state.profile.lock().clone();

    // Charger les mémoires persistantes pour le contexte cross-session
    let memories = state.history.get_memories().await.unwrap_or_default();
    let memory_context = if memories.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = memories.iter().map(|m| format!("- {} : {}", m.key, m.value)).collect();
        format!("MÉMOIRE PERSISTANTE (faits appris lors de conversations précédentes) :\n{}", items.join("\n"))
    };

    let is_deep_think = request.deep_think == Some(true);
    let prompt = if is_deep_think {
        build_deep_think_prompt(&request.message, &full_context, &memory_context, &history, &profile, agent.as_ref(), &agent_skills)
    } else {
        build_prompt(&request.message, &full_context, &memory_context, &history, &profile, agent.as_ref(), &agent_skills)
    };
    tracing::info!(
        "Prompt construit ({} caractères) — lancement de la génération...",
        prompt.len()
    );

    // 4. Génération en streaming avec boucle pour Function Calling
    let mut current_prompt = prompt;
    let mut final_response = String::new();
    let mut total_tokens = 0usize;
    let max_tool_loops = 5;
    let mut has_searched_web = should_search_web;

    // En mode DeepThink, on force la boucle de réflexion + recherche web
    if is_deep_think {
        has_searched_web = false;
    }

    for loop_idx in 0..max_tool_loops {
        let llm_state = state.llm.clone();
        let abort_flag = state.abort_generation.clone();
        let tx_clone = tx.clone();
        let conv_id_clone = conv_id.clone();
        let prompt_clone = current_prompt.clone();
        let has_searched_web_clone = has_searched_web;
        let is_deep_think_clone = is_deep_think;

        let (response_text, tokens, was_tool_aborted, was_reflection_aborted) = tokio::task::spawn_blocking(move || {
            let mut guard = llm_state.lock();
            let engine = guard
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("Moteur LLM non disponible"))?;

            tracing::info!("Début du prefill (loop {})...", loop_idx);
            let mut streamer = BatchStreamer::new();
            let mut tokens_count = 0usize;
            let mut accumulated = String::new();
            let mut is_tool_aborted = false;
            let mut is_reflection_aborted = false;

            // État DeepThink : parse les blocs <think>...</think>
            let mut in_think_block = false;
            let mut think_buffer = String::new();
            let mut think_phase = "thinking".to_string();
            let mut think_opened = false;
            let mut think_closed = false;

            let response = engine.generate_streaming(&prompt_clone, max_tokens, |token| {
                if abort_flag.load(Ordering::SeqCst) {
                    tracing::info!("Génération interrompue par l'utilisateur");
                    return false;
                }
                tokens_count += 1;
                accumulated.push_str(token);

                if accumulated.contains("</tool_call>") {
                    is_tool_aborted = true;
                    return false;
                }

                // --- Réflexion: Si l'IA indique ne pas savoir, déclencher la recherche web ---
                if !has_searched_web_clone && tokens_count < 80 && accumulated.len() > 15 {
                    let lower = accumulated.to_lowercase();
                    if lower.contains("pas d'information")
                        || lower.contains("aucune information")
                        || lower.contains("ne mentionne pas")
                        || lower.contains("je ne dispose pas")
                        || lower.contains("je n'ai pas d'information")
                        || lower.contains("je ne sais pas")
                        || lower.contains("le contexte fourni ne contient pas")
                    {
                        is_reflection_aborted = true;
                        return false; // Interrompre le LLM
                    }
                }

                // --- Mode DeepThink : state machine <think>...</think> ---
                if is_deep_think_clone {
                    if !think_opened && accumulated.contains("<think>") {
                        think_opened = true;
                        in_think_block = true;
                        let lower = accumulated.to_lowercase();
                        if lower.contains("décomposition") || lower.contains("sous-tâche") {
                            think_phase = "decomposition".to_string();
                        } else {
                            think_phase = "thinking".to_string();
                        }
                        // Ne pas émettre le token <think>
                        return true;
                    }
                    if think_opened && !think_closed && accumulated.contains("</think>") {
                        think_closed = true;
                        in_think_block = false;
                        // Envoyer le buffer restant avec phase synthesis
                        if !think_buffer.trim().is_empty() {
                            let _ = tx_clone.blocking_send(ChatEvent::DeepThinkStep {
                                phase: "synthesis".to_string(),
                                content: think_buffer.clone(),
                                conversation_id: conv_id_clone.clone(),
                            });
                            think_buffer.clear();
                        }
                        return true;
                    }
                    if in_think_block {
                        think_buffer.push_str(token);
                        // Émettre par chunks (phrase terminée ou buffer > 120 chars)
                        if (think_buffer.ends_with('.') || think_buffer.ends_with('\n') || think_buffer.len() > 120)
                            && !think_buffer.trim().is_empty()
                        {
                            let chunk = think_buffer.clone();
                            think_buffer.clear();
                            let _ = tx_clone.blocking_send(ChatEvent::DeepThinkStep {
                                phase: think_phase.clone(),
                                content: chunk,
                                conversation_id: conv_id_clone.clone(),
                            });
                        }
                        return true; // Ne pas émettre comme StreamToken
                    }
                }

                if let Some(batch) = streamer.push(token) {
                    let _ = tx_clone.blocking_send(ChatEvent::StreamToken {
                        token: batch,
                        conversation_id: conv_id_clone.clone(),
                    });
                }
                true
            });

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    let err_str = format!("{:?}", e);
                    if err_str.contains("OUT_OF_MEMORY") || err_str.contains("out of memory") {
                        anyhow::bail!(
                            "Mémoire GPU insuffisante pour cette requête. \
                             Essayez avec une question plus courte, ou passez en mode CPU dans les paramètres."
                        );
                    }
                    return Err(e);
                }
            };

            if let Some(remaining) = streamer.flush() {
                let _ = tx_clone.blocking_send(ChatEvent::StreamToken {
                    token: remaining,
                    conversation_id: conv_id_clone.clone(),
                });
            }

            // Flush residual think_buffer if the LLM stopped inside an unclosed <think> block
            if in_think_block && !think_buffer.trim().is_empty() {
                let _ = tx_clone.blocking_send(ChatEvent::DeepThinkStep {
                    phase: think_phase,
                    content: think_buffer,
                    conversation_id: conv_id_clone.clone(),
                });
            }

            Ok::<(String, usize, bool, bool), anyhow::Error>((response, tokens_count, is_tool_aborted, is_reflection_aborted))
        })
        .await??;

        total_tokens += tokens;

        // Si réflexion déclenchée, on efface le début de réponse et on lance la recherche web
        if was_reflection_aborted {
            tracing::info!("💡 Boucle de réflexion : L'IA manque d'informations. Déclenchement d'une recherche web profonde...");
            has_searched_web = true;
            
            let _ = tx.send(ChatEvent::WebSearchStatus {
                conversation_id: conv_id.clone(),
                status: "searching".to_string(),
                sources_count: 0,
            }).await;
            
            let web_results = match WebSearcher::new() {
                Ok(ws) => ws.search(&request.message, "general", 3).await.unwrap_or_default(),
                Err(e) => {
                    tracing::warn!("Erreur init web searcher: {}", e);
                    Vec::new()
                }
            };
            
            let mut new_web_ctx = String::new();
            for (i, res) in web_results.iter().take(3).enumerate() {
                new_web_ctx.push_str(&format!("Source Web {} ({}):\n{}\n\n", i + 1, res.url, res.content));
            }
            
            if !new_web_ctx.is_empty() {
                // Notifier client que la recherche est finie
                let _ = tx.send(ChatEvent::WebSearchStatus {
                    conversation_id: conv_id.clone(),
                    status: "done".to_string(),
                    sources_count: web_results.len(),
                }).await;
                
                // Mettre à jour le contexte et reconstruire le prompt
                let new_full_context = format!("{}\n\n{}", full_context, new_web_ctx);
                current_prompt = if is_deep_think {
                    build_deep_think_prompt(&request.message, &new_full_context, &memory_context, &history, &profile, agent.as_ref(), &agent_skills)
                } else {
                    build_prompt(&request.message, &new_full_context, &memory_context, &history, &profile, agent.as_ref(), &agent_skills)
                };
                
                // On efface la réponse générée pour que l'interface ne la garde pas
                let _ = tx.send(ChatEvent::StreamToken {
                    token: "\n*(Recherche web effectuée, je recalcule ma réponse...)*\n\n".to_string(),
                    conversation_id: conv_id.clone(),
                }).await;
                
                continue; // Relancer la génération avec le nouveau contexte
            }
        }

        final_response.push_str(&response_text);
        current_prompt.push_str(&response_text);

        if was_tool_aborted && response_text.contains("<tool_call>") {
            if let Some(start) = response_text.find("<tool_call>") {
                if let Some(end) = response_text.find("</tool_call>") {
                    let json_str = &response_text[start + 11..end];
                    tracing::info!("Tool call détecté : {}", json_str);
                    if let Ok(tool_call) = serde_json::from_str::<crate::llm::tools::ToolCall>(json_str) {
                        let _ = tx.send(ChatEvent::StreamToken {
                            token: format!("\n*(Exécution de l'outil {}...)*\n", tool_call.action),
                            conversation_id: conv_id.clone(),
                        }).await;
                        
                        let result = if tool_call.action == "delegate_task" {
                            let target_name = tool_call.args.get("agent_name").and_then(|v| v.as_str()).unwrap_or("");
                            let task = tool_call.args.get("task").and_then(|v| v.as_str()).unwrap_or("");
                            
                            let all_agents = state.workspace.list_agents().await.unwrap_or_default();
                            let target_agent = all_agents.into_iter().find(|a| a.name.to_lowercase() == target_name.to_lowercase());
                            
                            match target_agent {
                                Some(tgt) => {
                                    let req = ChatRequest {
                                        message: task.to_string(),
                                        conversation_id: None,
                                        max_tokens: Some(2000),
                                        agent_id: Some(tgt.id),
                                        deep_think: None,
                                    };
                                    let (sub_tx, mut sub_rx) = tokio::sync::mpsc::channel(100);
                                    let state_clone = state.clone();
                                    tokio::spawn(async move {
                                        while sub_rx.recv().await.is_some() {}
                                    });
                                    match Box::pin(process_chat(state_clone, req, sub_tx)).await {
                                        Ok(sub_res) => Ok(sub_res),
                                        Err(e) => Err(format!("Erreur lors de la délégation : {}", e))
                                    }
                                },
                                None => Err(format!("Agent '{}' introuvable.", target_name))
                            }
                        } else if tool_call.action == "load_skill" {
                            let skill_name = tool_call.args.get("skill_name").and_then(|v| v.as_str()).unwrap_or("");
                            let all_skills = state.workspace.list_skills().await.unwrap_or_default();
                            match all_skills.into_iter().find(|s| s.name.to_lowercase() == skill_name.to_lowercase()) {
                                Some(skill) => Ok(skill.content),
                                None => Err(format!("Skill '{}' introuvable.", skill_name))
                            }
                        } else {
                            let allowed_dir = agent.as_ref().and_then(|a| a.working_directory.clone());
                            crate::llm::tools::execute_tool(&tool_call, &allowed_dir).await
                        };
                        
                        let tool_result_str = match result {
                            Ok(res) => format!("\n<tool_result>\n{}\n</tool_result>\n", res),
                            Err(err) => format!("\n<tool_result>\nError: {}\n</tool_result>\n", err),
                        };
                        current_prompt.push_str(&tool_result_str);
                        continue;
                    } else {
                        current_prompt.push_str("\n<tool_result>\nError: Invalid JSON\n</tool_result>\n");
                        continue;
                    }
                }
            }
        }
        
        break; // Pas de tool call valide ou fin de génération
    }

    let elapsed = start_time.elapsed().as_millis() as u64;
    // Retirer les blocs <think>...</think> de la réponse finale (mode DeepThink)
    let final_response = strip_think_blocks(&final_response);
    let cleaned_response = truncate_gibberish(&strip_meta_notes(&final_response));

    // 5. Sauvegarder dans l'historique
    state
        .history
        .save_turn(&conv_id, &request.message, &cleaned_response)
        .await
        .ok();

    // 5b. Extraire et sauvegarder les mémoires persistantes
    let history_db = state.history.clone();
    let msg_for_mem = request.message.clone();
    let conv_id_mem = conv_id.clone();
    tokio::spawn(async move {
        extract_and_save_memories(&history_db, &msg_for_mem, &conv_id_mem).await;
    });

    // 6. Événement de fin
    let _ = tx
        .send(ChatEvent::GenerationDone {
            conversation_id: conv_id.clone(),
            full_response: cleaned_response,
            sources: all_sources,
            tokens_generated: total_tokens,
            time_ms: elapsed,
        })
        .await;

    state.touch_llm();
    Ok(conv_id)
}

// ─── Fonctions utilitaires (privées) ────────────────────────────────────────

/// Supprimer tous les blocs <think>...</think> d'un texte (mode DeepThink)
fn strip_think_blocks(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    loop {
        if let Some(open) = remaining.find("<think>") {
            result.push_str(&remaining[..open]);
            if let Some(close_rel) = remaining[open..].find("</think>") {
                remaining = &remaining[open + close_rel + "</think>".len()..];
                // Consommer un éventuel saut de ligne après </think>
                remaining = remaining.trim_start_matches('\n');
            } else {
                // Pas de fermeture : couper le reste
                break;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

fn format_web_context(results: &[WebResult]) -> String {
    if results.is_empty() {
        return String::new();
    }
    let mut context = String::from(
        "INFORMATIONS WEB (sources multiples à croiser — ne recopie PAS les en-têtes ni les URLs) :\n\
         Priorise les informations des sources officielles (.gouv.fr, .fr institutionnel).\n\
         Si les sources se contredisent, mentionne-le et privilégie les sources officielles.\n\n",
    );
    for (i, result) in results.iter().enumerate() {
        let reliability = if is_official_domain(&result.url) {
            "source officielle"
        } else {
            "source web"
        };
        let content_extract: String = result.content.chars().take(500).collect();
        let clean_content = truncate_at_sentence(&content_extract);
        context.push_str(&format!(
            "Source {} — {} ({}) :\n{}\n\n",
            i + 1,
            result.source_name,
            reliability,
            clean_content
        ));
    }
    context
}

fn truncate_at_sentence(text: &str) -> &str {
    let last_period = text.rfind(". ");
    let last_excl = text.rfind("! ");
    let last_quest = text.rfind("? ");
    let last_dot_end = if text.ends_with('.') {
        Some(text.len() - 1)
    } else {
        None
    };
    let candidates = [last_period, last_excl, last_quest, last_dot_end];
    if let Some(pos) = candidates.iter().filter_map(|&p| p).max() {
        &text[..=pos]
    } else {
        text
    }
}

fn is_official_domain(url: &str) -> bool {
    let official_tlds = [
        ".gouv.fr",
        "ameli.fr",
        "caf.fr",
        "urssaf.fr",
        "info-retraite.fr",
        "service-public.fr",
        "legifrance.gouv.fr",
        "defenseurdesdroits.fr",
        "justice.fr",
        "banque-france.fr",
    ];
    let lower = url.to_lowercase();
    official_tlds.iter().any(|d| lower.contains(d))
}

fn is_web_result_relevant(result: &WebResult, query: &str) -> bool {
    let query_lower = query.to_lowercase();
    let content_lower = result.content.to_lowercase();
    let title_lower = result.title.to_lowercase();
    let source_lower = result.source_name.to_lowercase();

    if source_lower.contains("duckduckgo") {
        return false;
    }
    if result.content.len() < 150 {
        return false;
    }

    let promo_markers = [
        "inscris-toi",
        "inscrivez-vous",
        "rejoignez notre formation",
        "nos formations",
        "bootcamp",
        "télécharge",
        "téléchargez",
        "demande de brochure",
        "finançable cpf",
    ];
    let promo_hits = promo_markers
        .iter()
        .filter(|m| content_lower.contains(*m))
        .count();
    if promo_hits >= 2 {
        return false;
    }

    let stop_words = [
        "quel", "quelle", "quels", "quelles", "est", "sont", "dans", "pour",
        "avec", "cette", "les", "des", "une", "que", "qui", "comment",
        "plus", "moins", "par", "sur", "aux", "fait", "faire", "peut",
        "entre", "comme", "mais", "aussi", "tout", "tous", "bien",
        "avoir", "récemment", "récente", "ancien", "ancienne",
        "être", "quoi", "quand", "pourquoi", "combien",
        "mon", "mes", "votre", "notre", "leur",
        "dois", "doit", "faut", "peux", "puis", "veut", "veux",
    ];

    let query_words: Vec<&str> = query_lower
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .filter(|w| w.len() > 3 && !stop_words.contains(w))
        .collect();

    if query_words.is_empty() {
        return true;
    }

    let title_matches = query_words
        .iter()
        .filter(|w| title_lower.contains(*w))
        .count();
    let content_matches = query_words
        .iter()
        .filter(|w| content_lower.contains(*w))
        .count();

    let weighted_score =
        (title_matches as f32 * 2.0 + content_matches as f32) / (query_words.len() as f32 * 2.0);

    weighted_score >= 0.3
}

fn strip_meta_notes(text: &str) -> String {
    let patterns: &[&str] = &[
        "(Note:",
        "(note:",
        "(NOTE:",
        "(Remarque:",
        "(remarque:",
        "(Since ",
        "(since ",
        "(NB:",
        "(NB :",
        "(Observation:",
        "(Internal note:",
        "(Context:",
        "(This ",
        "(this ",
        "(I ",
        "(As ",
    ];

    let mut result = text.to_string();
    for pattern in patterns {
        while let Some(start) = result.find(pattern) {
            let mut depth = 0;
            let mut end = start;
            for (i, c) in result[start..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = start + i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth != 0 {
                end = result[start..]
                    .find('\n')
                    .map(|p| start + p)
                    .unwrap_or(result.len());
            }
            result = format!("{}{}", &result[..start], result[end..].trim_start());
        }
    }

    let trailing_patterns = [
        "\nQuestion :",
        "\nQuestion avancée :",
        "\nQuestion avancée:",
        "\nQuestion:",
        "\nQuestion de suivi",
        "\nPour aller plus loin :",
        "\nPour aller plus loin:",
        "\nSuggestion :",
        "\nSuggestion:",
    ];
    for tp in &trailing_patterns {
        if let Some(pos) = result.find(tp) {
            result.truncate(pos);
        }
    }

    result.trim().to_string()
}

fn truncate_gibberish(text: &str) -> String {
    if text.len() < 50 {
        return text.to_string();
    }

    let mut sentences: Vec<&str> = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if matches!(c, '.' | '!' | '?') {
            let next = text[i + c.len_utf8()..].chars().next();
            if next.is_none() || next == Some(' ') || next == Some('\n') {
                sentences.push(&text[start..=i]);
                start = i + c.len_utf8();
            }
        }
    }
    if start < text.len() && text[start..].trim().len() > 5 {
        sentences.push(&text[start..]);
    }

    if sentences.is_empty() {
        return text.to_string();
    }

    let mut last_good_idx = sentences.len();
    for (idx, sentence) in sentences.iter().enumerate() {
        if is_sentence_gibberish(sentence) {
            last_good_idx = idx;
            tracing::debug!(
                "Gibberish détecté à la phrase {} : '{}'",
                idx,
                &sentence[..sentence.len().min(60)]
            );
            break;
        }
    }

    if last_good_idx == 0 {
        return text.to_string();
    }

    if last_good_idx >= sentences.len() {
        return text.to_string();
    }

    sentences[..last_good_idx].join("").trim().to_string()
}

fn is_sentence_gibberish(sentence: &str) -> bool {
    let trimmed = sentence.trim();
    if trimmed.len() < 10 {
        return false;
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() < 3 {
        return false;
    }

    let long_words = words.iter().filter(|w| w.chars().count() > 20).count();
    if long_words as f32 / words.len() as f32 > 0.2 {
        return true;
    }

    let weird_case = words
        .iter()
        .filter(|w| {
            let chars: Vec<char> = w.chars().collect();
            if chars.len() < 4 {
                return false;
            }
            chars[1..].iter().enumerate().any(|(i, c)| {
                c.is_uppercase() && i > 0 && chars[i].is_lowercase()
            })
        })
        .count();
    if weird_case > 2 && weird_case as f32 / words.len() as f32 > 0.3 {
        return true;
    }

    let comma_count = trimmed.matches(',').count();
    if comma_count > 5 && words.len() < 15 {
        let has_verb_indicator = trimmed.contains(" est ")
            || trimmed.contains(" sont ")
            || trimmed.contains(" a ")
            || trimmed.contains(" ont ")
            || trimmed.contains(" peut ")
            || trimmed.contains(" avec ");
        if !has_verb_indicator {
            return true;
        }
    }

    let repeated = words.windows(2).filter(|w| w[0] == w[1]).count();
    if repeated >= 3 {
        return true;
    }

    let non_french = trimmed
        .chars()
        .filter(|c| {
            !c.is_ascii_alphanumeric()
                && !c.is_whitespace()
                && !"àâäéèêëïîôùûüÿçœæÀÂÄÉÈÊËÏÎÔÙÛÜŸÇŒÆ.,;:!?-'\"()/«»€°—–\n".contains(*c)
        })
        .count();
    if trimmed.len() > 20 && non_french as f32 / trimmed.len() as f32 > 0.1 {
        return true;
    }

    false
}

fn extract_domain(url: &str) -> String {
    url.split("//")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .replace("www.", "")
        .replace("www2.", "")
        .to_lowercase()
}

// ─── Extraction de mémoires persistantes ────────────────────────────────────

use crate::history::sqlite::HistoryDb;
use std::sync::Arc;

/// Extraire des faits mémorisables du message utilisateur et les sauvegarder
async fn extract_and_save_memories(
    history: &Arc<HistoryDb>,
    user_message: &str,
    conversation_id: &str,
) {
    let q = user_message.to_lowercase();

    // Patterns d'auto-présentation
    let patterns: Vec<(&str, &[&str])> = vec![
        ("prenom", &[
            "je m'appelle ", "je m appelle ", "mon prénom est ", "mon prenom est ",
            "moi c'est ", "moi c est ",
        ]),
        ("profession", &[
            "je suis ", "je travaille comme ", "je travaille en tant que ",
            "mon métier ", "ma profession ",
        ]),
        ("localisation", &[
            "j'habite ", "j habite ", "je vis à ", "je vis a ",
            "je suis de ", "je réside ",
        ]),
        ("situation_familiale", &[
            "je suis marié", "je suis marie", "je suis célibataire", "je suis celibataire",
            "je suis divorcé", "je suis divorce",
            "je suis pacsé", "je suis pacse",
            "j'ai des enfants", "j ai des enfants",
        ]),
        ("statut", &[
            "je suis auto-entrepreneur", "je suis autoentrepreneur",
            "je suis salarié", "je suis salarie",
            "je suis fonctionnaire",
            "je suis étudiant", "je suis etudiant",
            "je suis retraité", "je suis retraite",
            "je suis au chômage", "je suis au chomage",
            "je suis indépendant", "je suis independant",
        ]),
    ];

    for (key, prefixes) in patterns {
        for prefix in prefixes.iter() {
            if let Some(pos) = q.find(prefix) {
                let start = pos + prefix.len();
                let value: String = user_message[start..]
                    .chars()
                    .take(80)
                    .take_while(|c| *c != '.' && *c != ',' && *c != '!' && *c != '?' && *c != '\n')
                    .collect();
                let value = value.trim().to_string();
                if value.len() >= 2 && value.len() <= 80 {
                    if let Err(e) = history.save_memory(key, &value, Some(conversation_id)).await {
                        tracing::debug!("Erreur sauvegarde mémoire '{}': {}", key, e);
                    } else {
                        tracing::info!("Mémoire persistante sauvée : {} = '{}'", key, value);
                    }
                    break;
                }
            }
        }
    }
}
