/// Source officielle interrogeable par Marianne
#[derive(Debug, Clone)]
pub struct OfficialSource {
    pub name: &'static str,
    pub search_url: &'static str,
    pub query_param: &'static str,
    pub content_selectors: &'static [&'static str],
    pub allowed_domains: &'static [&'static str],
    pub categories: &'static [&'static str],
}

// ═══════════════════════════════════════════════════════════════════════
// Sources officielles françaises, classées par domaine.
//
// Chaque source est taguée avec les catégories qu'elle couvre.
// `select_sources(category)` filtre automatiquement :
//   - Question "RSA" → catégorie "caf" → service-public.fr + caf.fr
//   - Question "Rust" → catégorie "rust" → aucune source officielle → DuckDuckGo
//   - Question "impôts" → catégorie "impots" → impots.gouv.fr + service-public.fr
// ═══════════════════════════════════════════════════════════════════════

pub const OFFICIAL_SOURCES: &[OfficialSource] = &[
    // ── Portail généraliste ──────────────────────────────────────────────
    OfficialSource {
        name: "Service-Public.fr",
        search_url: "https://www.service-public.fr/particuliers/recherche",
        query_param: "q",
        content_selectors: &[
            ".sp-article-body",
            ".article__body",
            ".fr-container article",
            "main .content",
            "article",
        ],
        allowed_domains: &["service-public.fr"],
        categories: &[
            "caf",
            "urssaf",
            "sante",
            "droit_travail",
            "logement",
            "retraite",
            "recours",
            "impots",
            "demarches",
            "famille",
            "consommation",
            "transport",
            "education",
            "emploi",
            "environnement",
            "general",
        ],
    },
    // ── Droit & Législation ──────────────────────────────────────────────
    OfficialSource {
        name: "Légifrance",
        search_url: "https://www.legifrance.gouv.fr/search/all",
        query_param: "query",
        content_selectors: &[
            ".content-article",
            ".article-content",
            "#content-article",
            ".main-content",
            "article",
        ],
        allowed_domains: &["legifrance.gouv.fr"],
        categories: &[
            "droit_travail",
            "code_civil",
            "penal",
            "procedure_judiciaire",
            "jurisprudence",
            "logement",
            "droit_immobilier",
            "famille",
            "consommation",
            "recours",
            "droit_numerique",
        ],
    },
    OfficialSource {
        // a corriger
        name: "Justice.fr",
        search_url: "https://www.justice.fr/recherche/all",
        query_param: "", // recherche par chemin : /recherche/all/{query}
        content_selectors: &[
            ".node__content",
            ".field--name-body",
            "article .content",
            "main article",
        ],
        allowed_domains: &["justice.fr"],
        categories: &[
            "recours",
            "procedure_judiciaire",
            "penal",
            "famille",
            "code_civil",
            "jurisprudence",
        ],
    },
    // ── Social & Prestations ─────────────────────────────────────────────
    OfficialSource {
        name: "CAF.fr",
        search_url: "https://www.caf.fr/allocataires/recherche",
        query_param: "q",
        content_selectors: &[
            ".field--name-body",
            ".node__content",
            ".article-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["caf.fr"],
        categories: &["caf", "logement", "famille"],
    },
    OfficialSource {
        name: "Ameli.fr",
        search_url: "https://www.ameli.fr/assure/recherche",
        query_param: "keys",
        content_selectors: &[
            ".field--name-body",
            ".article-body",
            ".block-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["ameli.fr"],
        categories: &["sante"],
    },
    OfficialSource {
        // a corriger
        name: "URSSAF",
        search_url: "https://www.autoentrepreneur.urssaf.fr/portail/accueil/recherche.html",
        query_param: "q",
        content_selectors: &[
            ".article-content",
            ".content-article",
            ".field--name-body",
            "article .content",
            "main",
        ],
        allowed_domains: &["urssaf.fr", "autoentrepreneur.urssaf.fr"],
        categories: &["urssaf", "comptabilite"],
    },
    OfficialSource {
        // a corriger
        name: "France Travail",
        search_url: "https://www.francetravail.fr/candidat/recherche.html",
        query_param: "q",
        content_selectors: &[
            ".article-body",
            ".content-article",
            ".block-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["francetravail.fr"],
        categories: &["emploi", "droit_travail", "chomage"],
    },
    // ── Fiscalité & Finance ──────────────────────────────────────────────
    OfficialSource {
        name: "Impots.gouv.fr",
        search_url: "https://www.impots.gouv.fr/recherche",
        query_param: "search_api_fulltext",
        content_selectors: &[
            ".field--name-body",
            ".node__content",
            ".article-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["impots.gouv.fr"],
        categories: &["impots", "comptabilite"],
    },
    OfficialSource {
        // a corriger
        name: "Économie.gouv.fr",
        search_url: "https://www.economie.gouv.fr/recherche",
        query_param: "search_api_fulltext",
        content_selectors: &[
            ".fr-container article",
            ".field--name-body",
            ".article-body",
            "article .content",
            "main article",
        ],
        allowed_domains: &["economie.gouv.fr"],
        categories: &[
            "impots",
            "finance_perso",
            "consommation",
            "consommateur",
            "comptabilite",
            "investissement",
        ],
    },
    OfficialSource {
        // a corriger
        name: "Banque de France",
        search_url: "https://www.banque-france.fr/fr/search",
        query_param: "keyword",
        content_selectors: &[
            ".field--name-body",
            ".article-body",
            ".block-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["banque-france.fr"],
        categories: &["finance_perso", "investissement", "crypto"],
    },
    // ── Retraite ─────────────────────────────────────────────────────────
    OfficialSource {
        // a corriger
        name: "Info-Retraite.fr",
        search_url: "https://www.info-retraite.fr/s/search",
        query_param: "search",
        content_selectors: &[
            ".article-content",
            ".slds-rich-text-editor__output",
            ".content-body",
            "article .content",
            "main article",
        ],
        allowed_domains: &["info-retraite.fr"],
        categories: &["retraite"],
    },
    // ── Logement ─────────────────────────────────────────────────────────
    OfficialSource {
        // a corriger
        name: "ANIL (logement)",
        search_url: "https://www.anil.org/outils/recherche",
        query_param: "s",
        content_selectors: &[
            ".entry-content",
            ".article-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["anil.org"],
        categories: &["logement", "droit_immobilier"],
    },
    // ── Données personnelles ─────────────────────────────────────────────
    OfficialSource {
        name: "CNIL",
        search_url: "https://www.cnil.fr/fr/recherche",
        query_param: "search_api_fulltext",
        content_selectors: &[
            ".field--name-body",
            ".node__content",
            ".article-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["cnil.fr"],
        categories: &["droit_numerique", "cybersecurite"],
    },
    // ── Éducation ────────────────────────────────────────────────────────
    OfficialSource {
        name: "Éducation.gouv.fr",
        search_url: "https://www.education.gouv.fr/recherche",
        query_param: "key",
        content_selectors: &[
            ".fr-container article",
            ".field--name-body",
            ".article-body",
            "article .content",
            "main article",
        ],
        allowed_domains: &["education.gouv.fr"],
        categories: &["education"],
    },
    // ── Environnement ────────────────────────────────────────────────────
    OfficialSource {
        name: "Écologie.gouv.fr",
        search_url: "https://www.ecologie.gouv.fr/recherche",
        query_param: "key",
        content_selectors: &[
            ".fr-container article",
            ".field--name-body",
            ".article-body",
            "article .content",
            "main article",
        ],
        allowed_domains: &["ecologie.gouv.fr"],
        categories: &["environnement"],
    },
    // ── Défenseur des droits ─────────────────────────────────────────────
    OfficialSource {
        name: "Défenseur des droits",
        search_url: "https://www.defenseurdesdroits.fr/recherche",
        query_param: "search_api_fulltext",
        content_selectors: &[
            ".field--name-body",
            ".node__content",
            ".article-content",
            "article .content",
            "main article",
        ],
        allowed_domains: &["defenseurdesdroits.fr"],
        categories: &["recours", "consommateur", "droit_numerique", "famille"],
    },
];

/// Sélectionner les sources pertinentes selon la catégorie détectée.
///
/// Retourne uniquement les sources dont les `categories` contiennent
/// la catégorie détectée. Si aucune source ne matche (ex: "rust",
/// "programmation"), la liste est vide et seul DuckDuckGo sera utilisé.
pub fn select_sources(category: &str) -> Vec<&'static OfficialSource> {
    OFFICIAL_SOURCES
        .iter()
        .filter(|s| s.categories.contains(&category))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_caf_routes_to_official_sources() {
        let sources = select_sources("caf");
        let names: Vec<&str> = sources.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Service-Public.fr"));
        assert!(names.contains(&"CAF.fr"));
        assert!(!names.contains(&"Légifrance")); // CAF ≠ Légifrance
    }

    #[test]
    fn test_programming_has_no_official_source() {
        assert!(select_sources("rust").is_empty());
        assert!(select_sources("programmation").is_empty());
        assert!(select_sources("python").is_empty());
        assert!(select_sources("web_dev").is_empty());
    }

    #[test]
    fn test_droit_travail_routes_to_legifrance() {
        let sources = select_sources("droit_travail");
        let names: Vec<&str> = sources.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Légifrance"));
        assert!(names.contains(&"Service-Public.fr"));
        assert!(names.contains(&"France Travail"));
    }

    #[test]
    fn test_impots_routes_correctly() {
        let sources = select_sources("impots");
        let names: Vec<&str> = sources.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Impots.gouv.fr"));
        assert!(names.contains(&"Service-Public.fr"));
        assert!(!names.contains(&"Ameli.fr")); // impôts ≠ santé
    }

    #[test]
    fn test_sante_routes_to_ameli() {
        let sources = select_sources("sante");
        let names: Vec<&str> = sources.iter().map(|s| s.name).collect();
        assert!(names.contains(&"Ameli.fr"));
        assert!(!names.contains(&"CAF.fr"));
    }

    #[test]
    fn test_all_sources_have_valid_config() {
        for source in OFFICIAL_SOURCES {
            assert!(!source.name.is_empty(), "Source sans nom");
            assert!(
                !source.search_url.is_empty(),
                "Source {} sans URL",
                source.name
            );
            assert!(
                !source.content_selectors.is_empty(),
                "Source {} sans sélecteurs",
                source.name
            );
            assert!(
                !source.allowed_domains.is_empty(),
                "Source {} sans domaines",
                source.name
            );
            assert!(
                !source.categories.is_empty(),
                "Source {} sans catégories",
                source.name
            );
        }
    }
}
