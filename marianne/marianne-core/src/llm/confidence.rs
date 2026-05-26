/// Évaluation de la confiance de la réponse RAG locale
/// Si le score est bas, Marianne déclenche la recherche web complémentaire

/// Seuil de base de confiance pour ne pas déclencher la recherche web
const BASE_CONFIDENCE_THRESHOLD: f32 = 0.45;

/// Message de refus pour les questions hors sujet
pub const OFF_TOPIC_RESPONSE: &str = "Je suis Marianne, spécialisée dans l'administration et les droits en France. Cette question ne fait pas partie de mes compétences. Posez-moi une question sur vos droits, démarches ou obligations en France ! 🇫🇷";

/// Détecter si une question est hors sujet (non liée à la vie en France)
/// Retourne true si la question est interdite et ne doit pas être envoyée au LLM
///
/// Double logique :
/// 1. Blacklist : mots-clés explicitement interdits → bloqué sauf si ancrage France
/// 2. Whitelist : si aucun terme admin/France détecté → bloqué (question trop éloignée)
pub fn is_off_topic(query: &str) -> bool {
    let q = query.to_lowercase();

    // Laisser passer les conversationnelles (bonjour, merci, etc.)
    if is_conversational(query) {
        return false;
    }

    // Questions trop courtes pour être analysées → laisser passer
    if q.len() < 12 {
        return false;
    }

    // ─── Blacklist : sujets explicitement interdits ────────────────────────
    let blocked_keywords = [
        // Programmation / informatique (technique pure)
        "coder", "programmer", "python", "javascript", "java ",
        "rust ", "html", "css", "sql", "api ", "github", "docker", "linux",
        "variable", "fonction", "class ", "debug",
        "compile", "framework", "frontend", "backend", "serveur web",
        "base de données", "machine learning",
        // Hacking / cybersécurité offensive
        "hack", "hacker", "pirater", "phishing", "exploit", "crack",
        "bruteforce", "ddos", "malware", "virus informatique",
        "ransomware", "injection sql", "faille de sécurité",
        // Cuisine / recettes
        "recette", "cuisiner", "ingrédient", "gâteau", "pâtisserie",
        "cheese cake", "cheesecake", "pizza", "dessert", "cuisson",
        "four à ", "moule à ", "pâte à ",
        // Jeux / divertissement
        "jeu vidéo", "playstation", "xbox", "nintendo", "fortnite", "minecraft",
        "film", "série tv", "netflix", "manga", "anime",
        // Sciences / maths pures
        "équation", "intégrale", "dérivée", "théorème", "physique quantique",
        "chimie organique", "atome", "molécule",
        // Investissement / spéculation
        "bitcoin", "ethereum", "crypto-monnaie", "cryptomonnaie", "trading",
        "spéculer", "nft",
    ];

    // ─── Whitelist : ancrage admin / vie en France ─────────────────────────
    let france_anchors = [
        // Géographie / identité France
        "france", "français", "française", "république", "état",
        // Juridique
        "droit", "loi", "décret", "article", "code du", "ordonnance",
        "légal", "juridique", "réglementaire", "obligation",
        "normatif", "normative", "législat", "abroger", "abrogé", "abrogation",
        "promulguer", "promulgation", "jurisprudence", "contentieux",
        "constitutionnel", "règlement", "circulaire", "arrêté",
        "officiel", "gazette", "journal officiel",
        // Administration
        "administration", "démarche", "formulaire", "cerfa", "dossier",
        "demande de", "procédure", "réclamation", "attestation",
        "service public", "fonctionnaire", "agent public", "collectivité",
        "ministère", "secrétariat", "autorité", "commission",
        // Organismes
        "impôt", "taxe", "fiscal", "caf", "rsa", "apl",
        "urssaf", "ameli", "cpam", "cnav", "ants", "préfecture", "mairie",
        "pôle emploi", "france travail", "sécurité sociale", "trésor public",
        // Travail
        "travail", "emploi", "contrat", "licenciement", "congé", "chômage",
        "salaire", "smic", "cdi", "cdd", "intérim", "employeur", "salarié",
        // Logement
        "locataire", "propriétaire", "bail", "loyer", "logement", "hlm",
        "expulsion", "préavis", "caution", "état des lieux",
        // Documents
        "carte d'identité", "passeport", "permis", "carte grise",
        "acte de naissance", "livret de famille", "extrait de casier",
        // Famille / état civil
        "mariage", "pacs", "divorce", "naissance", "décès",
        "garde", "pension alimentaire", "autorité parentale",
        // Succession / patrimoine
        "héritage", "succession", "donation", "notaire", "testament",
        // Social / santé
        "allocat", "aide sociale", "prime", "bourse", "handicap", "aah", "mdph",
        "maladie", "arrêt", "médecin", "hôpital", "mutuelle", "complémentaire",
        "retraite", "pension", "invalidité",
        // Justice
        "tribunal", "justice", "recours", "contestation", "plainte",
        "amende", "contravention", "infraction", "avocat", "huissier",
        // Consommation
        "consommation", "litige", "arnaque", "garantie",
        // Immigration
        "nationalité", "naturalisation", "titre de séjour", "visa",
        // Éducation
        "inscription", "scolarité", "école", "université",
        // Courrier
        "courrier", "lettre officielle", "lettre de", "recommandé",
        // Assurance
        "assurance", "sinistre", "indemnisation",
        // Vie quotidienne / société
        "jeune", "jeunes", "enfant", "parent", "famille",
        "citoyen", "citoyenne", "citoyenneté",
        "numérique", "internet", "données personnelles", "rgpd", "cnil",
        "intelligence artificielle", " ia ", "l'ia",
        "régulation", "réglementation", "encadrement",
        "société", "sociétal", "social",
        "mineur", "majeur", "adolescent",
        "éducation", "enseignement", "formation",
        // Marianne
        "marianne",
    ];

    let has_blocked = blocked_keywords.iter().any(|kw| q.contains(kw));
    let has_anchor = france_anchors.iter().any(|a| q.contains(a));

    // Règle 1 : mot-clé interdit ET pas d'ancrage France → bloqué
    if has_blocked && !has_anchor {
        return true;
    }

    // Règle 2 : aucun ancrage France sur une question très longue → bloqué
    // Seulement pour les questions clairement hors sujet (> 80 caractères sans aucun mot-clé lié à la France)
    // Les questions courtes/moyennes sont laissées au LLM qui sait refuser poliment
    if !has_anchor && q.len() > 80 {
        return true;
    }

    false
}

/// Détecter les questions conversationnelles/méta qui n'ont pas besoin de recherche web
pub fn is_conversational(query: &str) -> bool {
    let q = query.to_lowercase();
    let q_trimmed = q.trim();

    // Messages très courts (< 20 caractères) et non-question → conversationnel
    if q_trimmed.len() < 20 && !q_trimmed.contains("droit")
        && !q_trimmed.contains("loi") && !q_trimmed.contains("aide")
        && !q_trimmed.contains("caf") && !q_trimmed.contains("rsa")
        && !q_trimmed.contains("impôt") && !q_trimmed.contains("travail")
    {
        // Vérifier que c'est bien conversationnel et pas un mot-clé admin isolé
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
        // Sauf si le reste contient une vraie question admin
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

    // Questions méta sur Marianne
    let meta_patterns = [
        "qui es-tu",
        "qui es tu",
        "tu es qui",
        "c'est quoi marianne",
        "que peux-tu faire",
        "que peux tu faire",
        "qu'est-ce que tu peux",
        "qu'est ce que tu peux",
        "qu'est-ce que je peux te",
        "qu'est ce que je peux te",
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
        "tu t'appelle",
        "quel est ton nom",
        "ton nom",
        "c'est quoi ton",
        "tu es quoi",
    ];
    if meta_patterns.iter().any(|p| q.contains(p)) {
        return true;
    }

    // Remerciements / fin de conversation
    let closings = [
        "merci", "au revoir", "à bientôt", "a bientot", "ok merci",
        "parfait", "super", "génial", "d'accord", "entendu", "compris",
        "c'est noté", "c'est note", "top", "nickel", "formidable",
        "bonne journée", "bonne soirée", "bye",
    ];
    if closings.iter().any(|c| q_trimmed.starts_with(c)) && q_trimmed.len() < 60 {
        return true;
    }

    false
}

/// Évaluer la confiance à partir des résultats RAG avec seuil adaptatif par catégorie
pub fn evaluate_rag_confidence(
    rag_scores: &[f32],
    rag_context_len: usize,
    query_len: usize,
    category: &str,
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

    let should_search_web = confidence < adaptive_threshold(category);

    let reason = if should_search_web {
        format!(
            "Confiance faible ({:.0}%, seuil {:.0}% pour {}) — recherche web recommandée",
            confidence * 100.0,
            adaptive_threshold(category) * 100.0,
            category,
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

/// Seuil de confiance adaptatif par catégorie
/// Les catégories avec un bon corpus local ont un seuil plus élevé (plus exigeant)
/// Les catégories moins couvertes ont un seuil plus bas (recherche web plus facile)
fn adaptive_threshold(category: &str) -> f32 {
    match category {
        // Bien couvert par le corpus → seuil élevé, on fait confiance au local
        "caf" | "droit_travail" | "logement" | "chomage" => BASE_CONFIDENCE_THRESHOLD + 0.05,
        // Moyennement couvert → seuil standard
        "impots" | "sante" | "retraite" | "urssaf" | "identite" | "recours" => BASE_CONFIDENCE_THRESHOLD,
        // Peu couvert → seuil bas, recherche web plus fréquente
        "renovation" | "consommation" | "discrimination" | "institutions" | "statistiques" => BASE_CONFIDENCE_THRESHOLD - 0.10,
        // Catégorie inconnue → seuil standard
        _ => BASE_CONFIDENCE_THRESHOLD,
    }
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

/// Déterminer la catégorie de la question pour orienter la recherche web
pub fn detect_category(query: &str) -> &'static str {
    let q = query.to_lowercase();
    
    if q.contains("caf") || q.contains("allocation") || q.contains("apl") || q.contains("rsa") || q.contains("prime d'activité") {
        "caf"
    } else if q.contains("ameli") || q.contains("sécu") || q.contains("maladie") || q.contains("médecin") || q.contains("carte vitale") {
        "sante"
    } else if q.contains("urssaf") || q.contains("cotisation") || q.contains("auto-entrepreneur") || q.contains("micro") {
        "urssaf"
    } else if q.contains("chômage") || q.contains("chomage") || q.contains("pôle emploi") || q.contains("france travail") {
        "chomage"
    } else if q.contains("impôt") || q.contains("impot") || q.contains("taxe") || q.contains("fiscal") || q.contains("déclaration de revenus") {
        "impots"
    } else if q.contains("amende") || q.contains("contravention") || q.contains("radar") || q.contains("pv") {
        "amendes"
    } else if q.contains("passeport") || q.contains("carte d'identité") || q.contains("permis de conduire") || q.contains("carte grise") {
        "identite"
    } else if q.contains("travail") || q.contains("licenciement") || q.contains("contrat") || q.contains("cdi") || q.contains("cdd") || q.contains("smic") {
        "droit_travail"
    } else if q.contains("rénovation") || q.contains("renovation") || q.contains("maprimerénov") || q.contains("isolation") || q.contains("énergie") {
        "renovation"
    } else if q.contains("logement") || q.contains("bail") || q.contains("locataire") || q.contains("propriétaire") || q.contains("loyer") {
        "logement"
    } else if q.contains("retraite") || q.contains("pension") || q.contains("cnav") || q.contains("carrière") {
        "retraite"
    } else if q.contains("consommation") || q.contains("rappel produit") || q.contains("arnaque") || q.contains("litige") || q.contains("commerçant") {
        "consommation"
    } else if q.contains("discrimination") || q.contains("défenseur des droits") || q.contains("harcèlement") {
        "discrimination"
    } else if q.contains("surendettement") || q.contains("banque de france") || q.contains("droit au compte") || q.contains("ficp") {
        "surendettement"
    } else if q.contains("investissement") || q.contains("arnaque") && q.contains("financ") || q.contains("amf") || q.contains("bourse") || q.contains("crypto") {
        "fiscalite"
    } else if q.contains("député") || q.contains("sénateur") || q.contains("assemblée nationale") || q.contains("sénat") || q.contains("loi votée") || q.contains("gouvernement") {
        "institutions"
    } else if q.contains("statistique") || q.contains("insee") || q.contains("chiffre") || q.contains("population") || q.contains("inflation") {
        "statistiques"
    } else if q.contains("recours") || q.contains("tribunal") || q.contains("justice") || q.contains("plainte") || q.contains("procédure") {
        "recours"
    } else {
        "general"
    }
}
