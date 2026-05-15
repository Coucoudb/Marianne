// src-tauri/src/prompts/system.rs
use serde::{Deserialize, Serialize};

/// Prompt système principal de Marianne
pub const SYSTEM_PROMPT: &str = r#"Tu es Marianne, assistante administrative française. Tu aides les citoyens à comprendre leurs droits et démarches. Réponds en français, de manière claire et structurée. Si tu n'es pas certaine, dis-le."#;

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
) -> String {
    let mut prompt = String::new();

    // Prompt système au format Phi-3
    prompt.push_str("<|system|>\n");
    prompt.push_str(SYSTEM_PROMPT);
    prompt.push_str("<|end|>\n");

    // Contexte RAG si disponible (dans un tour user dédié)
    if !rag_context.is_empty() {
        prompt.push_str("<|user|>\n");
        prompt.push_str("Voici le contexte légal et réglementaire pertinent :\n");
        prompt.push_str(rag_context);
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        prompt.push_str("J'ai bien pris en compte ce contexte. Posez votre question.<|end|>\n");
    }

    // Historique de conversation (max 3 derniers échanges)
    let history_start = conversation_history.len().saturating_sub(3);
    for turn in &conversation_history[history_start..] {
        prompt.push_str("<|user|>\n");
        prompt.push_str(&turn.user);
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        prompt.push_str(&turn.assistant);
        prompt.push_str("<|end|>\n");
    }

    // Question actuelle
    prompt.push_str("<|user|>\n");
    prompt.push_str(user_question);
    prompt.push_str("<|end|>\n");
    prompt.push_str("<|assistant|>\n");

    prompt
}
