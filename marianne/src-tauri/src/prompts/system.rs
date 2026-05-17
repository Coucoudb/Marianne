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

/// Types de question détectés pour adapter le format de réponse
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuestionType {
    Factual,      // Question factuelle → réponse concise + source
    Procedure,    // Démarche → étapes numérotées avec checklist
    Letter,       // Demande de rédaction → format courrier
    Comparison,   // Question comparative → tableau
    General,      // Autre
}

/// Détecter le type de question pour adapter les instructions du prompt
pub fn detect_question_type(question: &str) -> QuestionType {
    let q = question.to_lowercase();

    // Rédaction de courrier/lettre
    if q.contains("courrier") || q.contains("lettre de") || q.contains("rédige")
        || q.contains("rédiger") || q.contains("écris") || q.contains("écrire")
        || q.contains("modèle de lettre") || q.contains("recommandé")
    {
        return QuestionType::Letter;
    }

    // Démarche / procédure multi-étapes
    if q.contains("comment faire") || q.contains("démarche") || q.contains("étapes")
        || q.contains("procédure") || q.contains("demande de") || q.contains("comment obtenir")
        || q.contains("comment demander") || q.contains("inscription") || q.contains("formulaire")
        || q.starts_with("comment ") && (q.contains("faire") || q.contains("obtenir") || q.contains("demander"))
    {
        return QuestionType::Procedure;
    }

    // Comparaison
    if q.contains("différence entre") || q.contains("comparer") || q.contains("versus")
        || q.contains(" vs ") || q.contains("ou bien") || q.contains("quel est le mieux")
        || (q.contains("cdi") && q.contains("cdd"))
        || (q.contains("rsa") && q.contains("prime"))
    {
        return QuestionType::Comparison;
    }

    // Question factuelle (qui, quoi, combien, quel montant, quelle durée)
    if q.starts_with("quel") || q.starts_with("combien") || q.starts_with("qui ")
        || q.contains("montant") || q.contains("durée") || q.contains("délai")
        || q.contains("plafond") || q.contains("conditions")
        || q.starts_with("est-ce que") || q.starts_with("est ce que")
    {
        return QuestionType::Factual;
    }

    QuestionType::General
}

/// Instructions supplémentaires selon le type de question
fn question_type_instructions(qt: QuestionType) -> &'static str {
    match qt {
        QuestionType::Factual => "\nFORMAT DE RÉPONSE : Réponds de façon concise et directe. Cite la source et le passage précis du contexte.",
        QuestionType::Procedure => "\nFORMAT DE RÉPONSE : Réponds sous forme d'étapes numérotées (1, 2, 3...). Pour chaque étape, indique le formulaire ou lien utile si disponible dans le contexte. Termine par les documents nécessaires.",
        QuestionType::Letter => "\nFORMAT DE RÉPONSE : Rédige le courrier au format officiel français : lieu et date, expéditeur, destinataire, objet, formule d'appel, corps, formule de politesse, signature. Utilise un ton formel.",
        QuestionType::Comparison => "\nFORMAT DE RÉPONSE : Structure ta réponse en comparant point par point les éléments demandés. Utilise un format clair avec des tirets ou un résumé structuré.",
        QuestionType::General => "",
    }
}

/// Résumer l'historique ancien en une phrase condensée
fn summarize_old_history(turns: &[ConversationTurn]) -> String {
    if turns.is_empty() {
        return String::new();
    }

    // Extraire les sujets clés des échanges précédents
    let topics: Vec<String> = turns.iter().map(|t| {
        // Prendre les 80 premiers caractères de la question utilisateur
        let summary: String = t.user.chars().take(80).collect();
        // Couper proprement au dernier espace
        if let Some(pos) = summary.rfind(' ') {
            summary[..pos].to_string()
        } else {
            summary
        }
    }).collect();

    format!(
        "Résumé des échanges précédents : l'utilisateur a posé des questions sur : {}.",
        topics.join(" ; ")
    )
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
    // Budget max pour le prompt (en caractères)
    // 4096 tokens ≈ ~12000 chars en français, on utilise 9000 pour garder ~3000 chars pour la génération
    const MAX_PROMPT_CHARS: usize = 9000;

    let question_type = detect_question_type(user_question);
    let type_instructions = question_type_instructions(question_type);

    let mut prompt = String::new();

    // Prompt système au format Phi-3
    prompt.push_str("<|system|>\n");
    prompt.push_str(SYSTEM_PROMPT);
    prompt.push_str(type_instructions);

    // Injecter le contexte utilisateur si renseigné
    let profile_context = profile.to_context_string();
    if !profile_context.is_empty() {
        prompt.push_str("\n\nCONTEXTE UTILISATEUR :\n");
        prompt.push_str(&profile_context);
    }

    prompt.push_str("<|end|>\n");

    // Calculer l'espace restant pour le contexte RAG
    let base_overhead = prompt.len() + user_question.len() + 200;
    let available_for_context = MAX_PROMPT_CHARS.saturating_sub(base_overhead + 600); // 600 pour historique

    // Contexte RAG si disponible — tronqué à une frontière de phrase si trop long
    if !rag_context.is_empty() {
        let truncated_context = truncate_at_boundary(rag_context, available_for_context);
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

    // Historique de conversation avec résumé intelligent
    if !conversation_history.is_empty() {
        let recent_count = 3.min(conversation_history.len());
        let old_count = conversation_history.len().saturating_sub(recent_count);

        // Résumer les anciens échanges (avant les 3 derniers) en une phrase
        if old_count > 0 {
            let summary = summarize_old_history(&conversation_history[..old_count]);
            if !summary.is_empty() && prompt.len() + summary.len() < MAX_PROMPT_CHARS - 500 {
                prompt.push_str("<|user|>\n");
                prompt.push_str(&summary);
                prompt.push_str("<|end|>\n");
                prompt.push_str("<|assistant|>\n");
                prompt.push_str("Noté, je prends en compte le contexte de notre conversation.<|end|>\n");
            }
        }

        // Injecter les 3 derniers tours complets
        let recent_start = conversation_history.len().saturating_sub(recent_count);
        for turn in &conversation_history[recent_start..] {
            if prompt.len() > MAX_PROMPT_CHARS - 300 {
                break;
            }
            prompt.push_str("<|user|>\n");
            prompt.push_str(&turn.user);
            prompt.push_str("<|end|>\n");
            prompt.push_str("<|assistant|>\n");
            // Tronquer les réponses longues de l'historique
            let assistant_text: String = turn.assistant.chars().take(400).collect();
            prompt.push_str(&assistant_text);
            prompt.push_str("<|end|>\n");
        }
    }

    // Question actuelle
    prompt.push_str("<|user|>\n");
    prompt.push_str(user_question);
    prompt.push_str("<|end|>\n");
    prompt.push_str("<|assistant|>\n");

    prompt
}

/// Tronquer un texte à une frontière de phrase (. ! ?) sans couper au milieu d'un mot
fn truncate_at_boundary(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    let truncated = &text[..max_chars];
    // Chercher la dernière fin de phrase dans la zone tronquée
    let last_period = truncated.rfind(". ");
    let last_newline = truncated.rfind("\n\n");
    let last_excl = truncated.rfind("! ");
    let last_quest = truncated.rfind("? ");

    let candidates = [last_period, last_newline, last_excl, last_quest];
    if let Some(pos) = candidates.iter().filter_map(|&p| p).max() {
        if pos > max_chars / 2 {
            // Couper à la fin de phrase si elle est dans la 2e moitié
            return text[..=pos].to_string();
        }
    }

    // Fallback : couper au dernier espace
    if let Some(pos) = truncated.rfind(' ') {
        text[..pos].to_string()
    } else {
        truncated.to_string()
    }
}
