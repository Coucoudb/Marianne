// src-tauri/src/prompts/system.rs
use serde::{Deserialize, Serialize};

/// Prompt système principal de Marianne
pub const SYSTEM_PROMPT: &str = r#"Tu es Marianne, une assistante virtuelle polyvalente. Tu adaptes tes réponses au contexte et aux compétences définies par l'utilisateur.

RÈGLES FONDAMENTALES :
- Réponds uniquement en français, de façon claire et accessible
- Appuie-toi sur le contexte fourni ci-dessous pour formuler ta réponse
- Tu dois OBLIGATOIREMENT citer les sources des données que tu utilises pour répondre, afin que l'utilisateur puisse vérifier l'information
- Ne donne pas de conseil médical ou financier personnalisé
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

    // Extraire les sujets clés des échanges précédents (question + extrait de réponse)
    let topics: Vec<String> = turns.iter().map(|t| {
        // Prendre les 80 premiers caractères de la question utilisateur
        let q_summary: String = t.user.chars().take(80).collect();
        let q_clean = if let Some(pos) = q_summary.rfind(' ') {
            q_summary[..pos].to_string()
        } else {
            q_summary
        };
        // Prendre les 60 premiers caractères de la réponse
        let a_summary: String = t.assistant.chars().take(60).collect();
        let a_clean = if let Some(pos) = a_summary.rfind(' ') {
            a_summary[..pos].to_string()
        } else {
            a_summary
        };
        format!("Q: {} / R: {}", q_clean, a_clean)
    }).collect();

    format!(
        "Résumé des échanges précédents :\n{}",
        topics.join("\n")
    )
}

/// Construire le prompt complet au format Phi-3-instruct chat template
///
/// Format Phi-3 : <|system|>\n{system}<|end|>\n<|user|>\n{user}<|end|>\n<|assistant|>\n
pub fn build_prompt(
    user_question: &str,
    rag_context: &str,
    memory_context: &str,
    conversation_history: &[ConversationTurn],
    profile: &crate::profile::UserProfile,
    agent: Option<&crate::workspace::agent::Agent>,
    skills: &[crate::workspace::skill::Skill],
) -> String {
    // Budget max pour le prompt (en caractères)
    // 4096 tokens ≈ ~12000 chars en français, on utilise 9000 pour garder ~3000 chars pour la génération
    const MAX_PROMPT_CHARS: usize = 9000;

    let question_type = detect_question_type(user_question);
    let type_instructions = question_type_instructions(question_type);

    let mut prompt = String::new();

    // Prompt système au format Phi-3
    prompt.push_str("<|system|>\n");
    if let Some(a) = agent {
        prompt.push_str(&a.system_prompt);
        if !a.tools.is_empty() {
            prompt.push_str("\n\nOUTILS DISPONIBLES :\nTu as accès aux outils suivants. Pour utiliser un outil, génère un bloc XML exact <tool_call>{\"action\": \"nom_outil\", \"args\": {\"arg1\": \"valeur\"}}</tool_call>. Tu dois attendre la réponse de l'outil avant de continuer. Exemple: <tool_call>{\"action\": \"list_dir\", \"args\": {\"path\": \"C:/\"}}</tool_call>\n");
            if a.tools.contains(&"read_file".to_string()) {
                prompt.push_str("- read_file : {\"action\": \"read_file\", \"args\": {\"path\": \"chemin_absolu\"}}\n");
            }
            if a.tools.contains(&"write_file".to_string()) {
                prompt.push_str("- write_file : {\"action\": \"write_file\", \"args\": {\"path\": \"chemin_absolu\", \"content\": \"contenu_texte\"}}\n");
            }
            if a.tools.contains(&"list_dir".to_string()) {
                prompt.push_str("- list_dir : {\"action\": \"list_dir\", \"args\": {\"path\": \"chemin_absolu_dossier\"}}\n");
            }
            if a.tools.contains(&"run_command".to_string()) {
                prompt.push_str("- run_command : {\"action\": \"run_command\", \"args\": {\"command\": \"commande_shell_a_executer\"}}\n");
            }
            if a.tools.contains(&"replace_file_content".to_string()) {
                prompt.push_str("- replace_file_content : {\"action\": \"replace_file_content\", \"args\": {\"path\": \"chemin_absolu\", \"old_text\": \"texte_exact_a_remplacer\", \"new_text\": \"nouveau_texte\"}}\n");
            }
            if a.tools.contains(&"grep_search".to_string()) {
                prompt.push_str("- grep_search : {\"action\": \"grep_search\", \"args\": {\"path\": \"chemin_absolu_dossier\", \"query\": \"expression_reguliere_ou_mot\"}}\n");
            }
            if a.tools.contains(&"delegate_task".to_string()) {
                prompt.push_str("- delegate_task : {\"action\": \"delegate_task\", \"args\": {\"agent_name\": \"nom_de_lagent\", \"task\": \"description de la tâche\"}}\n");
            }
            prompt.push_str("- load_skill : {\"action\": \"load_skill\", \"args\": {\"skill_name\": \"nom_du_skill\"}}\n");
        }
        if !skills.is_empty() {
            prompt.push_str("\n\nCOMPÉTENCES ASSIGNÉES (SKILLS) :\nVoici des bases de connaissances qui te sont affectées pour t'aider à accomplir ta tâche :\n");
            
            let working_dir = a.working_directory.clone();
            
            for skill in skills {
                let mut should_inject_full = false;
                
                if skill.scope.is_none() {
                    should_inject_full = true;
                } else if let Some(ref wd) = working_dir {
                    if let Some(ref pattern) = skill.scope {
                        let full_pattern = std::path::Path::new(wd).join(pattern).to_string_lossy().to_string();
                        if let Ok(mut paths) = glob::glob(&full_pattern) {
                            if paths.next().is_some() {
                                should_inject_full = true;
                            }
                        }
                    }
                }

                if should_inject_full {
                    prompt.push_str(&format!("\n--- [Skill: {}] ---\n{}\n", skill.name, skill.content));
                } else {
                    prompt.push_str(&format!("- {} : {} (non pertinent pour les fichiers actuels — utilise load_skill si besoin)\n", skill.name, skill.description));
                }
            }
            prompt.push_str("------------------\n");
        }
    } else {
        prompt.push_str(SYSTEM_PROMPT);
    }
    prompt.push_str(type_instructions);

    // Injecter le contexte utilisateur si renseigné
    let profile_context = profile.to_context_string();
    if !profile_context.is_empty() {
        prompt.push_str("\n\nCONTEXTE UTILISATEUR :\n");
        prompt.push_str(&profile_context);
    }

    // Injecter les mémoires persistantes (cross-session)
    if !memory_context.is_empty() {
        prompt.push_str("\n\n");
        prompt.push_str(memory_context);
    }

    prompt.push_str("<|end|>\n");

    // Calculer l'espace restant pour le contexte RAG
    let base_overhead = prompt.len() + user_question.len() + 200;
    let available_for_context = MAX_PROMPT_CHARS.saturating_sub(base_overhead + 600); // 600 pour historique

    // Contexte RAG si disponible — tronqué à une frontière de phrase si trop long
    if !rag_context.is_empty() {
        let truncated_context = truncate_at_boundary(rag_context, available_for_context);
        prompt.push_str("<|user|>\n");
        prompt.push_str("Voici le contexte pertinent pour ma question. Réponds à partir de ces informations et cite tes sources :\n");
        prompt.push_str(&truncated_context);
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        prompt.push_str("Compris. Je répondrai à partir du contexte fourni en citant mes sources.<|end|>\n");
    } else {
        prompt.push_str("<|user|>\n");
        prompt.push_str("Aucun contexte n'est disponible pour cette question. Si tu ne connais pas la réponse avec certitude, dis-le honnêtement.\n");
        prompt.push_str("<|end|>\n");
        prompt.push_str("<|assistant|>\n");
        prompt.push_str("Compris. Sans contexte, je resterai prudente dans ma réponse.<|end|>\n");
    }

    // Historique de conversation avec résumé intelligent
    if !conversation_history.is_empty() {
        let recent_count = 5.min(conversation_history.len());
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
