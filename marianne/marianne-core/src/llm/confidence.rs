/// Évaluation de la confiance de la réponse RAG locale
/// Si le score est bas, Marianne déclenche la recherche web complémentaire

/// Seuil de base de confiance pour ne pas déclencher la recherche web
const BASE_CONFIDENCE_THRESHOLD: f32 = 0.45;

/// Message de refus pour les questions hors sujet (Désactivé)
pub const OFF_TOPIC_RESPONSE: &str = "Je suis un agent IA, comment puis-je vous aider ?";

/// L'IA est désormais généraliste : aucune question n'est hors sujet.
pub fn is_off_topic(_query: &str) -> bool {
    false
}

/// Détecter les questions conversationnelles/méta qui n'ont pas besoin de recherche web
pub fn is_conversational(query: &str) -> bool {
    let q = query.to_lowercase();
    let q_trimmed = q.trim();

    // Messages très courts (< 20 caractères) -> souvent conversationnel
    if q_trimmed.len() < 20 {
        let conv_short = [
            "bonjour", "salut", "coucou", "hello", "bonsoir", "hey",
            "merci", "ok", "oui", "non", "d'accord", "parfait", "super",
            "au revoir", "bye", "à bientôt", "a bientot", "bonne journée",
            "bonne soirée", "bon week-end", "ça va", "ca va", "bien",
            "comment vas", "comment tu", "quoi de neuf",
        ];
        if conv_short.iter().any(|c| q_trimmed.contains(c)) {
            return true;
        }
    }

    // Salutations (même dans une phrase plus longue)
    let greetings = [
        "bonjour", "salut", "coucou", "hello", "bonsoir", "hey ",
        "bonne journée", "bonne soirée",
    ];
    if greetings.iter().any(|g| q_trimmed.starts_with(g)) && q_trimmed.len() < 80 {
        let after = greetings.iter()
            .filter_map(|g| q_trimmed.strip_prefix(g))
            .next()
            .unwrap_or("")
            .trim_start_matches(|c: char| c == ',' || c == '!' || c == '.' || c.is_whitespace());
        if after.is_empty() || after.len() < 15 {
            return true;
        }
    }

    // Questions sur l'état / bien-être
    let wellbeing = [
        "comment vas-tu", "comment vas tu", "comment tu vas",
        "comment allez-vous", "comment allez vous",
        "ça va", "ca va", "tu vas bien", "vous allez bien",
        "la forme", "en forme", "quoi de neuf", "quoi de beau",
    ];
    if wellbeing.iter().any(|w| q_trimmed.contains(w)) {
        return true;
    }

    // Questions méta sur l'IA
    let meta_patterns = [
        "qui es-tu",
        "qui es tu",
        "tu es qui",
        "c'est quoi ton",
        "que peux-tu faire",
        "que peux tu faire",
        "qu'est-ce que tu peux",
        "qu'est ce que tu peux",
        "quelles questions",
        "quel type de question",
        "comment tu fonctionne",
        "comment fonctionne",
        "à quoi tu sers",
        "a quoi tu sers",
        "tu sais faire quoi",
        "tu fais quoi",
        "aide-moi",
        "aide moi",
        "tes capacités",
        "tes fonctionnalités",
        "présente-toi",
        "présente toi",
        "ton rôle",
        "ton role",
        "tu t'appelles",
        "quel est ton nom",
    ];
    if meta_patterns.iter().any(|p| q.contains(p)) {
        return true;
    }

    // Remerciements / fin de conversation
    let closings = [
        "merci", "au revoir", "à bientôt", "a bientot", "ok merci",
        "parfait", "super", "génial", "d'accord", "entendu", "compris",
        "c'est noté", "c'est note", "top", "nickel", "formidable",
        "bye",
    ];
    if closings.iter().any(|c| q_trimmed.starts_with(c)) && q_trimmed.len() < 60 {
        return true;
    }

    false
}

/// Évaluer la confiance à partir des résultats RAG
pub fn evaluate_rag_confidence(
    rag_scores: &[f32],
    rag_context_len: usize,
    query_len: usize,
    _category: &str,
) -> ConfidenceResult {
    if rag_scores.is_empty() {
        return ConfidenceResult {
            score: 0.0,
            reason: "Aucun résultat RAG trouvé".to_string(),
            should_search_web: true,
        };
    }

    let best_score = rag_scores.iter().cloned().fold(0.0f32, f32::max);
    let avg_score = rag_scores.iter().sum::<f32>() / rag_scores.len() as f32;

    // Facteurs de confiance
    let mut confidence: f32 = 0.0;

    // Score du meilleur résultat (0-0.4)
    confidence += best_score * 0.4;

    // Moyenne des scores (0-0.3)
    confidence += avg_score * 0.3;

    // Nombre de résultats pertinents (0-0.15)
    let relevant_count = rag_scores.iter().filter(|&&s| s > 0.3).count();
    confidence += (relevant_count.min(3) as f32 / 3.0) * 0.15;

    // Ratio contexte/question (0-0.15)
    if query_len > 0 {
        let ratio = (rag_context_len as f32 / query_len as f32).min(10.0) / 10.0;
        confidence += ratio * 0.15;
    }

    let should_search_web = confidence < BASE_CONFIDENCE_THRESHOLD;

    let reason = if should_search_web {
        format!(
            "Confiance faible ({:.0}%, seuil {:.0}%) — recherche web recommandée",
            confidence * 100.0,
            BASE_CONFIDENCE_THRESHOLD * 100.0,
        )
    } else {
        format!("Confiance suffisante ({:.0}%)", confidence * 100.0)
    };

    ConfidenceResult {
        score: confidence,
        reason,
        should_search_web,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfidenceResult {
    pub score: f32,
    pub reason: String,
    pub should_search_web: bool,
}

/// Détecter si l'utilisateur reformule sa question (signe d'insatisfaction)
/// Retourne true si le message actuel est une reformulation du message précédent
pub fn detect_reformulation(current: &str, previous: &str) -> bool {
    if previous.is_empty() || current.is_empty() {
        return false;
    }

    let cur_lower = current.to_lowercase();
    let prev_lower = previous.to_lowercase();

    // Même question exacte → pas une reformulation, c'est un retry
    if cur_lower == prev_lower {
        return false;
    }

    // Extraire les mots significatifs (> 3 chars)
    let cur_words: std::collections::HashSet<&str> = cur_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3)
        .collect();
    let prev_words: std::collections::HashSet<&str> = prev_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3)
        .collect();

    if cur_words.is_empty() || prev_words.is_empty() {
        return false;
    }

    // Overlap élevé mais pas identique = reformulation
    let intersection = cur_words.intersection(&prev_words).count();
    let union = cur_words.union(&prev_words).count();
    let jaccard = intersection as f32 / union as f32;

    // Jaccard entre 0.4 et 0.9 = reformulation probable
    jaccard > 0.4 && jaccard < 0.9
}

/// Détecter si l'utilisateur est satisfait (remerciement positif)
pub fn detect_satisfaction(message: &str) -> bool {
    let q = message.to_lowercase().trim().to_string();
    let positive = [
        "merci", "parfait", "super", "génial", "excellent", "top",
        "nickel", "formidable", "c'est clair", "bien compris",
        "c'est noté", "très bien", "ok merci", "merci beaucoup",
        "merci bien", "je comprends",
    ];
    positive.iter().any(|p| q.contains(p))
}

/// Déterminer la catégorie générale de la question (simplifié)
pub fn detect_category(_query: &str) -> &'static str {
    "general"
}
