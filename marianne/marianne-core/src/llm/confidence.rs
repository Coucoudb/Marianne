//! Évaluation de la confiance de la réponse RAG locale
//! Si le score est bas, Marianne déclenche la recherche web complémentaire

/// Seuil de base de confiance pour ne pas déclencher la recherche web
const BASE_CONFIDENCE_THRESHOLD: f32 = 0.55;

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

/// Détecter si une question nécessite obligatoirement une recherche web
/// (questions temporelles, actualité, événements récents, dates spécifiques)
pub fn requires_web_search(query: &str) -> bool {
    let q = query.to_lowercase();

    // Dates récentes ou spécifiques (ex: "loi du 26 mai 2026", "décret de janvier 2025")
    let date_patterns = [
        "202", "loi du ", "décret du ", "arrêté du ", "circulaire du ",
        "texte du ", "réforme du ", "réforme de ",
    ];
    let has_date_ref = date_patterns.iter().any(|p| q.contains(p));

    // Marqueurs de temporalité/actualité
    let temporal_markers = [
        "récemment", "récente", "récent", "dernière", "dernier",
        "nouveau", "nouvelle", "nouveaux", "nouvelles",
        "actualité", "actualite", "actualités",
        "cette année", "ce mois", "cette semaine",
        "en vigueur", "entré en vigueur", "entre en vigueur",
        "dernières nouvelles", "dernieres nouvelles",
        "mis à jour", "mise à jour", "mis a jour",
        "changement", "changements",
        "dernière réforme", "derniere reforme",
        "à partir de", "a partir de",
        "depuis le ", "à compter du", "a compter du",
    ];
    let has_temporal = temporal_markers.iter().any(|m| q.contains(m));

    // Questions sur des montants/barèmes qui changent annuellement
    let annual_data = [
        "montant du smic", "smic horaire", "smic mensuel",
        "plafond sécurité sociale", "barème", "bareme",
        "plafond caf", "montant rsa", "montant apl",
        "montant prime", "taux d'intérêt", "taux d'usure",
        "inflation",
    ];
    let has_annual = annual_data.iter().any(|m| q.contains(m));

    has_date_ref || has_temporal || has_annual
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

    // Score du meilleur résultat (0-0.35)
    confidence += best_score * 0.35;

    // Moyenne des scores (0-0.25)
    confidence += avg_score * 0.25;

    // Nombre de résultats vraiment pertinents (score > 0.5) — (0-0.2)
    let highly_relevant = rag_scores.iter().filter(|&&s| s > 0.5).count();
    let somewhat_relevant = rag_scores.iter().filter(|&&s| s > 0.3).count();
    confidence += (highly_relevant.min(3) as f32 / 3.0) * 0.2;

    // Ratio contexte/question (0-0.1)
    if query_len > 0 {
        let ratio = (rag_context_len as f32 / query_len as f32).min(10.0) / 10.0;
        confidence += ratio * 0.1;
    }

    // Malus : si les scores sont faibles malgré beaucoup de résultats
    // (le RAG retourne du bruit, pas de l'info pertinente)
    if best_score < 0.4 && somewhat_relevant == 0 {
        confidence *= 0.6; // Réduire de 40%
    } else if best_score < 0.5 && highly_relevant == 0 {
        confidence *= 0.8; // Réduire de 20%
    }

    // Bonus : contexte substantiel ET scores élevés
    if rag_context_len > 500 && best_score > 0.7 && highly_relevant >= 2 {
        confidence += 0.1;
    }

    // Plafonner à 1.0
    confidence = confidence.min(1.0);

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

/// Déterminer la catégorie générale de la question pour le filtrage RAG.
/// Scan rapide par keywords — exécuté à chaque requête utilisateur.
pub fn detect_category(query: &str) -> &'static str {
    let q = query.to_lowercase();

    // ── Droits & Administration ──────────────────────────────────────
    if q.contains("caf") || q.contains("apl") || q.contains("rsa") || q.contains("prime d'activité") || q.contains("allocation") {
        return "caf";
    }
    if q.contains("urssaf") || q.contains("auto-entrepreneur") || q.contains("micro-entreprise") || q.contains("cotisations sociales") {
        return "urssaf";
    }
    if q.contains("ameli") || q.contains("cpam") || q.contains("sécurité sociale") || q.contains("arrêt maladie") || q.contains("mutuelle") {
        return "sante";
    }
    if q.contains("licenciement") || q.contains("prud'hommes") || q.contains("prudhommes") || q.contains("contrat de travail") || q.contains("heures supplémentaires") || q.contains("cdd") || q.contains("cdi") {
        return "droit_travail";
    }
    if q.contains("bail") || q.contains("locataire") || q.contains("propriétaire") || q.contains("loyer") || q.contains("expulsion") || q.contains("préavis") || q.contains("dépôt de garantie") {
        return "logement";
    }
    if q.contains("retraite") || q.contains("pension") || q.contains("trimestre") || q.contains("cnav") || q.contains("agirc") {
        return "retraite";
    }
    if q.contains("recours") || q.contains("contestation") || q.contains("médiateur") || q.contains("mediateur") {
        return "recours";
    }
    if q.contains("impôt") || q.contains("impot") || q.contains("déclaration de revenu") || q.contains("taxe foncière") || q.contains("taxe d'habitation") || q.contains("tva") {
        return "impots";
    }

    // ── Droit étendu ─────────────────────────────────────────────────
    if q.contains("tribunal") || q.contains("assignation") || q.contains("audience") || q.contains("jugement") || q.contains("appel") || q.contains("cassation") {
        return "procedure_judiciaire";
    }
    if q.contains("code civil") || q.contains("responsabilité civile") || q.contains("dommages et intérêts") || q.contains("prescription") {
        return "code_civil";
    }
    if q.contains("rétractation") || q.contains("retractation") || q.contains("garantie légale") || q.contains("pratiques commerciales") {
        return "consommation";
    }
    if q.contains("infraction") || q.contains("délit") || q.contains("contravention") || q.contains("garde à vue") || q.contains("procureur") || q.contains("plainte") {
        return "penal";
    }
    if q.contains("divorce") || q.contains("pension alimentaire") || q.contains("garde d'enfant") || q.contains("autorité parentale") || q.contains("pacs") || q.contains("adoption") || q.contains("succession") {
        return "famille";
    }
    if q.contains("jurisprudence") || q.contains("cour de cassation") || q.contains("conseil d'état") || q.contains("conseil d'etat") {
        return "jurisprudence";
    }
    if q.contains("copropriété") || q.contains("permis de construire") || q.contains("urbanisme") || q.contains("cadastre") || q.contains("notaire") {
        return "droit_immobilier";
    }
    if q.contains("rgpd") || q.contains("cnil") || q.contains("données personnelles") || q.contains("droit à l'oubli") || q.contains("cyberharcèlement") {
        return "droit_numerique";
    }

    // ── Finance & Économie ───────────────────────────────────────────
    if q.contains("épargne") || q.contains("crédit bancaire") || q.contains("prêt immobilier") || q.contains("surendettement") || q.contains("banque de france") {
        return "finance_perso";
    }
    if q.contains("bourse") || q.contains("action") && q.contains("investir") || q.contains("etf") || q.contains("pea") || q.contains("assurance-vie") || q.contains("dividende") {
        return "investissement";
    }
    if q.contains("bitcoin") || q.contains("ethereum") || q.contains("blockchain") || q.contains("nft") || q.contains("defi") || (q.contains("crypto") && !q.contains("cryptograph")) {
        return "crypto";
    }
    if q.contains("comptabilité") || q.contains("bilan comptable") || q.contains("liasse fiscale") || q.contains("amortissement") {
        return "comptabilite";
    }

    // ── Vie quotidienne ──────────────────────────────────────────────
    if q.contains("permis de conduire") || q.contains("carte grise") || q.contains("sncf") || q.contains("covoiturage") || q.contains("assurance auto") {
        return "transport";
    }
    if q.contains("bourse étudiante") || q.contains("inscription") && q.contains("université") || q.contains("cpf") || q.contains("apprentissage") || q.contains("formation professionnelle") {
        return "education";
    }
    if q.contains("chômage") || q.contains("chomage") || q.contains("france travail") || q.contains("pôle emploi") || q.contains("pole emploi") || q.contains("are") && q.contains("emploi") || q.contains("lettre de motivation") {
        return "emploi";
    }
    if q.contains("carte d'identité") || q.contains("passeport") || q.contains("acte de naissance") || q.contains("état civil") || q.contains("préfecture") {
        return "demarches";
    }
    if q.contains("recyclage") || q.contains("énergie renouvelable") || q.contains("maprimerénov") || q.contains("isolation thermique") || q.contains("diagnostic énergétique") {
        return "environnement";
    }

    // ── Programmation & Tech ─────────────────────────────────────────
    if q.contains("algorithme") || q.contains("complexité") || q.contains("structure de données") || q.contains("programmation dynamique") || q.contains("récursion") || q.contains("tri rapide") {
        return "algorithmique";
    }
    if q.contains("rust") || q.contains("ownership") || q.contains("borrowing") || q.contains("lifetime") || q.contains("cargo") && q.contains("crate") {
        return "rust";
    }
    if q.contains("python") || q.contains("pandas") || q.contains("numpy") || q.contains("django") || q.contains("flask") || q.contains("pip install") {
        return "python";
    }
    if q.contains("html") || q.contains("css") || q.contains("javascript") || q.contains("react") || q.contains("svelte") || q.contains("angular") || q.contains("node.js") || q.contains("frontend") || q.contains("backend") {
        return "web_dev";
    }
    if q.contains("docker") || q.contains("kubernetes") || q.contains("ci/cd") || q.contains("github actions") || q.contains("terraform") || q.contains("déploiement") {
        return "devops";
    }
    if q.contains("sql") || q.contains("postgresql") || q.contains("mongodb") || q.contains("redis") || q.contains("base de données") || q.contains("orm") {
        return "base_donnees";
    }
    if q.contains("machine learning") || q.contains("deep learning") || q.contains("réseau de neurones") || q.contains("llm") || q.contains("fine-tuning") || q.contains("intelligence artificielle") {
        return "ia_ml";
    }
    if q.contains("programmation") || q.contains("code source") || q.contains("compilateur") || q.contains("débugger") || q.contains("développement logiciel") || q.contains("langage de programmation") {
        return "programmation";
    }

    // ── Cybersécurité & Hacking ──────────────────────────────────────
    if q.contains("pentest") || q.contains("test d'intrusion") || q.contains("exploit") || q.contains("ctf") || q.contains("reverse engineering") || q.contains("bug bounty") || q.contains("hacking") {
        return "hacking";
    }
    if q.contains("cybersécurité") || q.contains("pare-feu") || q.contains("firewall") || q.contains("antivirus") || q.contains("zero trust") || q.contains("soc") && q.contains("sécurité") {
        return "cybersecurite";
    }
    if q.contains("tcp/ip") || q.contains("dns") || q.contains("vpn") || q.contains("proxy") || q.contains("routeur") || q.contains("wireshark") || q.contains("nat") {
        return "reseau";
    }
    if q.contains("cryptographie") || q.contains("chiffrement") || q.contains("aes") || q.contains("rsa") && q.contains("chiffr") || q.contains("ssl") || q.contains("tls") || q.contains("pki") {
        return "crypto_secu";
    }

    // ── Sciences & Culture ───────────────────────────────────────────
    if q.contains("histoire") || q.contains("guerre mondiale") || q.contains("révolution") || q.contains("moyen âge") || q.contains("antiquité") || q.contains("napoléon") {
        return "histoire";
    }
    if q.contains("géographie") || q.contains("continent") || q.contains("capitale") || q.contains("population") && q.contains("pays") {
        return "geographie";
    }
    if q.contains("physique") || q.contains("chimie") || q.contains("biologie") || q.contains("mathématiques") || q.contains("formule") || q.contains("expérience scientifique") {
        return "sciences";
    }
    if q.contains("philosophie") || q.contains("éthique") || q.contains("métaphysique") || q.contains("existentialisme") || q.contains("nietzsche") || q.contains("sartre") {
        return "philosophie";
    }
    if q.contains("littérature") || q.contains("roman") || q.contains("poésie") || q.contains("auteur") && q.contains("livre") {
        return "litterature";
    }
    if q.contains("cinéma") || q.contains("musique") || q.contains("musée") || q.contains("patrimoine") || q.contains("architecture") {
        return "culture_generale";
    }

    "general"
}
