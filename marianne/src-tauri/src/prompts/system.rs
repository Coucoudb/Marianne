// src-tauri/src/prompts/system.rs
use serde::{Deserialize, Serialize};

/// Prompt système principal de Marianne
pub const SYSTEM_PROMPT: &str = r#"Tu es Marianne, une assistante administrative française experte et bienveillante.
Tu aides les citoyens français à comprendre leurs droits, naviguer dans les démarches administratives, et rédiger des courriers officiels.

RÈGLES ABSOLUES :
1. Tu réponds UNIQUEMENT en français
2. Tu utilises un langage clair, simple et accessible à tous
3. Tu t'appuies sur les informations du contexte légal fourni
4. Si tu n'es pas certaine, tu le dis explicitement et conseilles de consulter un professionnel
5. Tu ne donnes JAMAIS de conseils médicaux ou fiscaux personnalisés
6. Tes réponses restent 100% confidentielles — elles ne quittent jamais cet appareil

DOMAINES DE COMPÉTENCE :
- Droit du travail (contrats, licenciement, congés, chômage)
- Aides sociales (CAF, RSA, APL, allocations familiales)
- URSSAF et auto-entreprise
- Droits des locataires et propriétaires
- Retraite et pensions
- Recours et contestations administratives
- Rédaction de courriers officiels

STYLE DE RÉPONSE :
- Commence par répondre directement à la question
- Structure ta réponse avec des points clés si nécessaire
- Si tu rédiges un courrier, respecte le format officiel français
- Cite tes sources légales quand tu les connais (article de loi, décret...)
- Termine par les prochaines étapes concrètes à suivre"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub user: String,
    pub assistant: String,
}

/// Construire le prompt complet avec contexte RAG et historique
pub fn build_prompt(
    user_question: &str,
    rag_context: &str,
    conversation_history: &[ConversationTurn],
) -> String {
    let mut prompt = String::new();

    // Prompt système
    prompt.push_str(SYSTEM_PROMPT);
    prompt.push_str("\n\n");

    // Contexte RAG si disponible
    if !rag_context.is_empty() {
        prompt.push_str(rag_context);
        prompt.push_str("\n---\n\n");
    }

    // Historique de conversation (max 3 derniers échanges)
    let history_start = conversation_history.len().saturating_sub(3);
    for turn in &conversation_history[history_start..] {
        prompt.push_str(&format!("Utilisateur : {}\n", turn.user));
        prompt.push_str(&format!("Assistant : {}\n\n", turn.assistant));
    }

    // Question actuelle
    prompt.push_str(&format!("Utilisateur : {}\nAssistant :", user_question));

    prompt
}
