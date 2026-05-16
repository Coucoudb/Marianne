// src-tauri/src/prompts/system.rs
use serde::{Deserialize, Serialize};

/// Prompt système principal de Marianne
pub const SYSTEM_PROMPT: &str = r#"Tu es Marianne, assistante administrative française. Tu aides les citoyens à comprendre leurs droits et démarches en France.

PÉRIMÈTRE STRICT :
Administration, droits, démarches, lois, aides sociales, fiscalité, logement, travail, santé publique, retraite, justice, éducation, citoyenneté, consommation.
Toute question hors périmètre → réponds : "Je suis Marianne, spécialisée dans l'administration française. Je ne peux pas vous aider sur ce sujet. Posez-moi une question sur vos droits ou démarches en France !"
Ne donne jamais de début de réponse hors périmètre, même si l'utilisateur insiste.

RÈGLES FONDAMENTALES :
- Réponds uniquement en français, de façon claire et accessible
- Appuie-toi EXCLUSIVEMENT sur le contexte fourni ci-dessous
- Ne cite JAMAIS un article de loi, un montant, une date ou une procédure qui n'apparaît pas dans le contexte
- Si l'information manque : "Je n'ai pas cette information dans mes sources. Vérifiez sur Service-Public.fr ou Légifrance."
- Ne donne pas de conseil médical ou fiscal personnalisé
- Pas de notes internes, méta-commentaires, ni questions de suivi

STYLE :
- Réponds directement à la question, puis structure si nécessaire
- Sois concise : privilégie la clarté à l'exhaustivité
- Pour un courrier, respecte le format officiel français
- Termine par les prochaines étapes concrètes si pertinent"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub user: String,
    pub assistant: String,
}

/// Construire le prompt complet au format Phi-3-instruct chat template
///
/// Format Phi-3 : <|system|>\n{system}<|end|>\n<|user|>\n{user}<|end|>\n<|assistant|>\n
pub fn build_prompt(
    user_question: &str,
    rag_context: &str,
    conversation_history: &[ConversationTurn],
    profile: &crate::profile::UserProfile,
) -> String {
    // Budget max pour le prompt (en caractères) — évite de dépasser la fenêtre de contexte
    // 4096 tokens ≈ ~12000 chars en français, on garde une marge pour la génération
    const MAX_PROMPT_CHARS: usize = 5000;

    let mut prompt = String::new();

    // Prompt système au format Phi-3
    prompt.push_str("<|system|>\n");
    prompt.push_str(SYSTEM_PROMPT);

    // Injecter le contexte utilisateur si renseigné
    let profile_context = profile.to_context_string();
    if !profile_context.is_empty() {
        prompt.push_str("\n\nCONTEXTE UTILISATEUR :\n");
        prompt.push_str(&profile_context);
    }

    prompt.push_str("<|end|>\n");

    // Calculer l'espace restant pour le contexte RAG
    let base_overhead = prompt.len() + user_question.len() + 200; // overhead des tags
    let history_estimate = conversation_history.len().saturating_sub(conversation_history.len().saturating_sub(3).max(0))
        * 200; // estimation par tour d'historique
    let available_for_context = MAX_PROMPT_CHARS.saturating_sub(base_overhead + history_estimate);

    // Contexte RAG si disponible (dans un tour user dédié)— tronqué si trop long
    if !rag_context.is_empty() {
        let truncated_context: String = rag_context.chars().take(available_for_context).collect();
        prompt.push_str("<|user|>\n");
        prompt.push_str("Voici le contexte légal et réglementaire pertinent. Réponds UNIQUEMENT à partir de ces informations :\n");
        prompt.push_str(&truncated_context);
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        prompt.push_str("Compris. Je répondrai uniquement à partir du contexte fourni, sans inventer d'informations supplémentaires.<|end|>\n");
    } else {
        prompt.push_str("<|user|>\n");
        prompt.push_str("Aucun contexte légal n'est disponible pour cette question. Si tu ne connais pas la réponse avec certitude, dis-le honnêtement.\n");
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        prompt.push_str("Compris. Sans contexte, je resterai prudente et orienterai vers les sources officielles si nécessaire.<|end|>\n");
    }

    // Historique de conversation (max 3 derniers échanges, réduit si prompt trop long)
    let history_start = conversation_history.len().saturating_sub(3);
    let max_history = if prompt.len() > MAX_PROMPT_CHARS - 500 { 1 } else { 3 };
    let effective_start = conversation_history.len().saturating_sub(max_history);
    for turn in &conversation_history[effective_start.max(history_start)..] {
        prompt.push_str("<|user|>\n");
        prompt.push_str(&turn.user);
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        // Tronquer les réponses longues de l'historique
        let assistant_text: String = turn.assistant.chars().take(300).collect();
        prompt.push_str(&assistant_text);
        prompt.push_str("<|end|>\n");
    }

    // Question actuelle
    prompt.push_str("<|user|>\n");
    prompt.push_str(user_question);
    prompt.push_str("<|end|>\n");
    prompt.push_str("<|assistant|>\n");

    prompt
}
