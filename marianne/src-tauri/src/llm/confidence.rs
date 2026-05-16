/// Évaluation de la confiance de la réponse RAG locale
/// Si le score est bas, Marianne déclenche la recherche web complémentaire

/// Seuil minimum de confiance pour ne pas déclencher la recherche web
pub const CONFIDENCE_THRESHOLD: f32 = 0.45;

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
        // Programmation / informatique
        "code", "coder", "programmer", "python", "javascript", "java ",
        "rust ", "html", "css", "sql", "api ", "github", "docker", "linux",
        "algorithme", "variable", "fonction", "class ", "debug",
        "compile", "framework", "frontend", "backend", "serveur web",
        "base de données", "machine learning", "intelligence artificielle",
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
        // Marianne
        "marianne",
    ];

    let has_blocked = blocked_keywords.iter().any(|kw| q.contains(kw));
    let has_anchor = france_anchors.iter().any(|a| q.contains(a));

    // Règle 1 : mot-clé interdit ET pas d'ancrage France → bloqué
    if has_blocked && !has_anchor {
        return true;
    }

    // Règle 2 : aucun ancrage France sur une question substantielle → bloqué
    // (questions > 40 caractères sans aucun terme admin/France = hors sujet)
    if !has_anchor && q.len() > 40 {
        return true;
    }

    false
}

/// Détecter les questions conversationnelles/méta qui n'ont pas besoin de recherche web
pub fn is_conversational(query: &str) -> bool {
    let q = query.to_lowercase();

    // Salutations
    let greetings = ["bonjour", "salut", "coucou", "hello", "bonsoir", "hey"];
    if greetings.iter().any(|g| q.starts_with(g)) && q.len() < 60 {
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
    ];
    if meta_patterns.iter().any(|p| q.contains(p)) {
        return true;
    }

    // Remerciements / fin de conversation
    let closings = ["merci", "au revoir", "à bientôt", "a bientot", "ok merci", "parfait", "super"];
    if closings.iter().any(|c| q.starts_with(c)) && q.len() < 50 {
        return true;
    }

    false
}

/// Évaluer la confiance à partir des résultats RAG
pub fn evaluate_rag_confidence(
    rag_scores: &[f32],
    rag_context_len: usize,
    query_len: usize,
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

    let should_search_web = confidence < CONFIDENCE_THRESHOLD;

    let reason = if should_search_web {
        format!(
            "Confiance faible ({:.0}%) — recherche web recommandée",
            confidence * 100.0
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
