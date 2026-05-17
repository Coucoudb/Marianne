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
    // Security: limit input message length to prevent DoS
    if request.message.len() > 10_000 {
        return Err("Message trop long (max 10 000 caractères).".to_string());
    }
    if request.message.trim().is_empty() {
        return Err("Message vide.".to_string());
    }

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

    // 0b. Messages conversationnels — skip RAG et recherche web
    let is_conv = is_conversational(&request.message);

    // 1. Pipeline RAG : trouver le contexte pertinent (skip pour conversationnel)
    let (rag_context, sources, rag_scores) = if is_conv {
        (String::new(), Vec::new(), Vec::new())
    } else {
        let retriever = Retriever::new(state.vector_store.clone());
        match retriever.retrieve(&request.message, 3).await {
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
        }
    };

    // 1b. Évaluer la confiance et déclencher la recherche web si nécessaire
    let confidence = evaluate_rag_confidence(
        &rag_scores,
        rag_context.len(),
        request.message.len(),
    );

    // Ne pas afficher le badge confiance/recherche web pour les messages conversationnels
    if !is_conv {
        let _ = window.emit(
            "confidence-info",
            ConfidenceInfo {
                conversation_id: conv_id.clone(),
                score: confidence.score,
                reason: confidence.reason.clone(),
                web_search_triggered: confidence.should_search_web,
            },
        );
    }

    let (web_context, all_sources) = if confidence.should_search_web && !is_conv {
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

        // Filtrer les résultats web non pertinents AVANT injection dans le contexte
        let relevant_web: Vec<WebResult> = web_results.into_iter()
            .filter(|r| is_web_result_relevant(r, &request.message))
            .collect();

        let web_ctx = format_web_context(&relevant_web);

        // Dédupliquer les sources par domaine (pas 2× "Service-Public.fr")
        let mut seen_domains = std::collections::HashSet::new();
        let web_srcs: Vec<String> = relevant_web.iter()
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
                sources_count: relevant_web.len(),
            },
        );

        let mut combined_sources = sources;
        combined_sources.extend(web_srcs);
        // Dédupliquer aussi les sources RAG+web par domaine
        let mut final_domains = std::collections::HashSet::new();
        combined_sources.retain(|url| {
            let domain = extract_domain(url);
            final_domains.insert(domain)
        });
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
        });

        // Gérer spécifiquement les erreurs CUDA OOM
        let response = match response {
            Ok(r) => r,
            Err(e) => {
                let err_str = format!("{:?}", e);
                if err_str.contains("OUT_OF_MEMORY") || err_str.contains("out of memory") {
                    anyhow::bail!("Mémoire GPU insuffisante pour cette requête. Essayez avec une question plus courte, ou passez en mode CPU dans les paramètres.");
                }
                return Err(e);
            }
        };

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
    let cleaned_response = truncate_gibberish(&strip_meta_notes(&full_response.0));

    // 5. Sauvegarder dans l'historique
    state
        .history
        .save_turn(
            &conv_id,
            &request.message,
            &cleaned_response,
        )
        .await
        .ok();

    // 6. Émettre l'événement de fin
    let _ = window.emit(
        "generation-done",
        GenerationDone {
            conversation_id: conv_id.clone(),
            full_response: cleaned_response,
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

    let mut context = String::from(
        "INFORMATIONS WEB (sources multiples à croiser — ne recopie PAS les en-têtes ni les URLs) :\n\
         Priorise les informations des sources officielles (.gouv.fr, .fr institutionnel).\n\
         Si les sources se contredisent, mentionne-le et privilégie les sources officielles.\n\n"
    );

    for (i, result) in results.iter().enumerate() {
        let reliability = if is_official_domain(&result.url) {
            "source officielle"
        } else {
            "source web"
        };
        // Limiter à 500 chars par source pour éviter de polluer le contexte du LLM
        let content_extract: String = result.content.chars().take(500).collect();
        // Tronquer à la dernière phrase complète si possible
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

/// Tronquer un texte à la dernière phrase complète (se terminant par . ! ou ?)
fn truncate_at_sentence(text: &str) -> &str {
    // Chercher la dernière fin de phrase dans le texte
    let last_period = text.rfind(". ");
    let last_excl = text.rfind("! ");
    let last_quest = text.rfind("? ");
    let last_dot_end = if text.ends_with('.') { Some(text.len() - 1) } else { None };

    let candidates = [last_period, last_excl, last_quest, last_dot_end];
    if let Some(pos) = candidates.iter().filter_map(|&p| p).max() {
        &text[..=pos]
    } else {
        text
    }
}

/// Déterminer si une URL est une source officielle française
fn is_official_domain(url: &str) -> bool {
    let official_tlds = [
        ".gouv.fr", "ameli.fr", "caf.fr", "urssaf.fr",
        "info-retraite.fr", "service-public.fr", "legifrance.gouv.fr",
        "defenseurdesdroits.fr", "justice.fr", "banque-france.fr",
    ];
    let lower = url.to_lowercase();
    official_tlds.iter().any(|d| lower.contains(d))
}
/// Vérifier qu'un résultat web a un lien sémantique minimal avec la question.
/// Évite d'afficher des sources hors sujet (ex: page Covid pour une question juridique).
fn is_web_result_relevant(result: &WebResult, query: &str) -> bool {
    let query_lower = query.to_lowercase();
    let content_lower = result.content.to_lowercase();
    let title_lower = result.title.to_lowercase();
    let source_lower = result.source_name.to_lowercase();

    // Rejeter les résultats provenant de DuckDuckGo lui-même (scraping raté)
    if source_lower.contains("duckduckgo") {
        return false;
    }

    // Contenu trop court = page vide ou erreur
    if result.content.len() < 150 {
        return false;
    }

    // Rejeter le contenu promotionnel / formations privées
    let promo_markers = [
        "inscris-toi", "inscrivez-vous", "rejoignez notre formation",
        "nos formations", "bootcamp", "télécharge", "téléchargez",
        "demande de brochure", "finançable cpf",
    ];
    let content_lower_promo = result.content.to_lowercase();
    let promo_hits = promo_markers.iter().filter(|m| content_lower_promo.contains(*m)).count();
    if promo_hits >= 2 {
        return false;
    }

    // Extraire les mots significatifs de la question (> 3 chars, pas des mots vides)
    let stop_words = [
        "quel", "quelle", "quels", "quelles", "est", "sont", "dans", "pour",
        "avec", "cette", "les", "des", "une", "que", "qui", "comment",
        "plus", "moins", "par", "sur", "aux", "fait", "faire", "peut",
        "entre", "comme", "mais", "aussi", "tout", "tous", "bien",
        "avoir", "r\u{00e9}cemment", "r\u{00e9}cente", "ancien", "ancienne",
        "\u{00ea}tre", "quoi", "quand", "pourquoi", "combien",
        "mon", "mes", "votre", "notre", "leur",
        "dois", "doit", "faut", "peux", "puis", "veut", "veux",
    ];

    let query_words: Vec<&str> = query_lower
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'')
        .filter(|w| w.len() > 3 && !stop_words.contains(w))
        .collect();

    if query_words.is_empty() {
        return true; // Pas assez de mots pour filtrer
    }

    // Le titre est un meilleur signal de pertinence que le contenu brut
    let title_matches = query_words.iter()
        .filter(|w| title_lower.contains(*w))
        .count();
    let content_matches = query_words.iter()
        .filter(|w| content_lower.contains(*w))
        .count();

    // Score pond\u{00e9}r\u{00e9} : titre vaut 2x, contenu 1x
    let weighted_score = (title_matches as f32 * 2.0 + content_matches as f32) /
        (query_words.len() as f32 * 2.0);

    weighted_score >= 0.3
}
/// Supprimer les méta-commentaires et notes internes générés par le LLM.
/// Ex: "(Note: Since no context was provided...)", "(Remarque interne...)"
/// Supprime aussi les questions parasites en fin de réponse ("Question : ...")
fn strip_meta_notes(text: &str) -> String {
    let patterns: &[&str] = &[
        "(Note:", "(note:", "(NOTE:",
        "(Remarque:", "(remarque:",
        "(Since ", "(since ",
        "(NB:", "(NB :",
        "(Observation:",
        "(Internal note:",
        "(Context:",
        "(This ", "(this ",
        "(I ", "(As ",
    ];

    let mut result = text.to_string();
    for pattern in patterns {
        while let Some(start) = result.find(pattern) {
            // Trouver la parenthèse fermante correspondante
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
                // Parenthèse non fermée — supprimer jusqu'à la fin de la ligne
                end = result[start..].find('\n').map(|p| start + p).unwrap_or(result.len());
            }
            result = format!("{}{}", &result[..start], result[end..].trim_start());
        }
    }

    // Supprimer les questions parasites en fin de réponse
    let trailing_patterns = [
        "\nQuestion :", "\nQuestion avancée :", "\nQuestion avancée:",
        "\nQuestion:", "\nQuestion de suivi",
        "\nPour aller plus loin :", "\nPour aller plus loin:",
        "\nSuggestion :", "\nSuggestion:",
    ];
    for tp in &trailing_patterns {
        if let Some(pos) = result.find(tp) {
            result.truncate(pos);
        }
    }

    result.trim().to_string()
}

/// Détecter et tronquer le gibberish dans la réponse du LLM.
/// Analyse par fenêtre glissante de phrases : si une phrase a trop de mots inexistants
/// ou de patterns incohérents, on tronque la réponse à la dernière bonne phrase.
fn truncate_gibberish(text: &str) -> String {
    if text.len() < 50 {
        return text.to_string();
    }

    // Découper en phrases
    let mut sentences: Vec<&str> = Vec::new();
    let mut start = 0;
    for (i, c) in text.char_indices() {
        if matches!(c, '.' | '!' | '?') {
            // Vérifier que c'est une fin de phrase (suivi d'espace ou fin)
            let next = text[i+c.len_utf8()..].chars().next();
            if next.is_none() || next == Some(' ') || next == Some('\n') {
                sentences.push(&text[start..=i]);
                start = i + c.len_utf8();
            }
        }
    }
    // Ajouter le reste s'il y en a
    if start < text.len() && text[start..].trim().len() > 5 {
        sentences.push(&text[start..]);
    }

    if sentences.is_empty() {
        return text.to_string();
    }

    // Analyser chaque phrase pour détecter le gibberish
    let mut last_good_idx = sentences.len(); // par défaut tout est bon
    for (idx, sentence) in sentences.iter().enumerate() {
        if is_sentence_gibberish(sentence) {
            last_good_idx = idx;
            tracing::debug!("Gibberish détecté à la phrase {} : '{}'", idx, &sentence[..sentence.len().min(60)]);
            break;
        }
    }

    if last_good_idx == 0 {
        // Même la première phrase est du gibberish — fallback
        return text.to_string();
    }

    if last_good_idx >= sentences.len() {
        return text.to_string();
    }

    // Tronquer à la dernière bonne phrase
    sentences[..last_good_idx].join("").trim().to_string()
}

/// Déterminer si une phrase individuelle est du gibberish
fn is_sentence_gibberish(sentence: &str) -> bool {
    let trimmed = sentence.trim();
    if trimmed.len() < 10 {
        return false; // Trop court pour juger
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() < 3 {
        return false;
    }

    // Critère 1 : mots très longs (concaténation anormale)
    let long_words = words.iter().filter(|w| w.chars().count() > 20).count();
    if long_words as f32 / words.len() as f32 > 0.2 {
        return true;
    }

    // Critère 2 : alternance anormale majuscule/minuscule au milieu des mots
    let weird_case = words.iter().filter(|w| {
        let chars: Vec<char> = w.chars().collect();
        if chars.len() < 4 { return false; }
        // Un mot avec une majuscule au milieu (pas un acronyme, pas le début)
        chars[1..].iter().enumerate().any(|(i, c)| {
            c.is_uppercase() && i > 0 && chars[i].is_lowercase()
        })
    }).count();
    if weird_case > 2 && weird_case as f32 / words.len() as f32 > 0.3 {
        return true;
    }

    // Critère 3 : densité de virgules/fragments (phrase hachée sans verbe)
    let comma_count = trimmed.matches(',').count();
    if comma_count > 5 && words.len() < 15 {
        // Beaucoup de virgules sur peu de mots = liste incohérente
        let has_verb_indicator = trimmed.contains(" est ") || trimmed.contains(" sont ")
            || trimmed.contains(" a ") || trimmed.contains(" ont ")
            || trimmed.contains(" peut ") || trimmed.contains(" avec ");
        if !has_verb_indicator {
            return true;
        }
    }

    // Critère 4 : répétitions de mots consécutifs
    let repeated = words.windows(2).filter(|w| w[0] == w[1]).count();
    if repeated >= 3 {
        return true;
    }

    // Critère 5 : caractères non-français excessifs
    let non_french = trimmed.chars().filter(|c| {
        !c.is_ascii_alphanumeric() && !c.is_whitespace()
            && !"àâäéèêëïîôùûüÿçœæÀÂÄÉÈÊËÏÎÔÙÛÜŸÇŒÆ.,;:!?-'\"()/«»€°—–\n".contains(*c)
    }).count();
    if trimmed.len() > 20 && non_french as f32 / trimmed.len() as f32 > 0.1 {
        return true;
    }

    false
}

/// Extraire le domaine principal d'une URL (pour déduplication)
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