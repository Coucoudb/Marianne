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

/// Liste blanche des sources officielles françaises
pub const OFFICIAL_SOURCES: &[OfficialSource] = &[
    OfficialSource {
        name: "Service-Public.fr",
        search_url: "https://www.service-public.gouv.fr/particuliers/recherche",
        query_param: "keyword",
        content_selectors: &["article.article-body", ".fiche-content", ".bloc-detail", ".sp-article-body"],
        allowed_domains: &["service-public.gouv.fr", "service-public.fr"],
        categories: &["general", "droit_travail", "logement", "famille", "retraite", "identite", "vehicule", "amendes"],
    },
    OfficialSource {
        name: "Légifrance",
        search_url: "https://www.legifrance.gouv.fr/search",
        query_param: "query",
        content_selectors: &[".article-content", ".content-article", ".texte-article"],
        allowed_domains: &["legifrance.gouv.fr"],
        categories: &["droit_travail", "logement", "recours", "general", "amendes"],
    },
    OfficialSource {
        name: "URSSAF",
        search_url: "https://www.urssaf.fr/accueil/recherche.html",
        query_param: "src_terms[0].term",
        content_selectors: &[".wysiwyg", ".content-page", "main article"],
        allowed_domains: &["urssaf.fr", "autoentrepreneur.urssaf.fr"],
        categories: &["urssaf", "autoentrepreneur"],
    },
    OfficialSource {
        name: "CAF.fr",
        search_url: "https://www.caf.fr/allocataires/recherche",
        query_param: "search",
        content_selectors: &[".field-body", ".content-zone", "article .body"],
        allowed_domains: &["caf.fr"],
        categories: &["caf", "famille", "logement"],
    },
    OfficialSource {
        name: "Ameli.fr",
        search_url: "https://www.ameli.fr/assure/recherche",
        query_param: "text",
        content_selectors: &[".field--name-body", ".article-content", ".bloc-text"],
        allowed_domains: &["ameli.fr"],
        categories: &["sante", "maladie", "arret_travail"],
    },
    OfficialSource {
        name: "France Travail",
        search_url: "https://candidat.francetravail.fr/offres/recherche",
        query_param: "motsCles",
        content_selectors: &[".article-content", ".wysiwyg-content", ".description"],
        allowed_domains: &["francetravail.fr", "candidat.francetravail.fr"],
        categories: &["chomage", "droit_travail"],
    },
    OfficialSource {
        name: "Impots.gouv.fr",
        search_url: "https://www.impots.gouv.fr/recherche",
        query_param: "key",
        content_selectors: &[".field--name-body", ".content-area", "main article"],
        allowed_domains: &["impots.gouv.fr"],
        categories: &["impots", "fiscalite"],
    },
    OfficialSource {
        name: "Info-Retraite",
        search_url: "https://www.info-retraite.fr/portail-info/sites/PortailInformationnel/home/resultats-de-recherche.html",
        query_param: "searchedText",
        content_selectors: &[".article-content", ".text-content", "main .content"],
        allowed_domains: &["info-retraite.fr"],
        categories: &["retraite"],
    },
    OfficialSource {
        name: "ANTS",
        search_url: "https://ants.gouv.fr/rechercher",
        query_param: "q",
        content_selectors: &[".content-body", ".field--name-body", "main article"],
        allowed_domains: &["ants.gouv.fr"],
        categories: &["identite", "vehicule"],
    },
    OfficialSource {
        name: "France Rénov",
        search_url: "https://france-renov.gouv.fr/rechercher",
        query_param: "mot_cle",
        content_selectors: &[".content-body", ".field-body", "main article"],
        allowed_domains: &["france-renov.gouv.fr"],
        categories: &["logement", "renovation"],
    },
    OfficialSource {
        name: "Défenseur des Droits",
        search_url: "https://www.defenseurdesdroits.fr/recherche",
        query_param: "keys",
        content_selectors: &[".field--name-body", ".content-area", "main article"],
        allowed_domains: &["defenseurdesdroits.fr"],
        categories: &["recours", "discrimination"],
    },
    OfficialSource {
        name: "Justice.fr",
        search_url: "https://www.justice.fr/recherche/all",
        query_param: "",
        content_selectors: &[".content-body", ".field--name-body", "main article"],
        allowed_domains: &["justice.fr"],
        categories: &["recours", "justice", "amendes"],
    },
    OfficialSource {
        name: "RappelConso",
        search_url: "https://rappel.conso.gouv.fr/categorie/0/1",
        query_param: "",
        content_selectors: &[".content", "main article", ".product-info", ".card"],
        allowed_domains: &["rappel.conso.gouv.fr"],
        categories: &["consommation", "sante"],
    },
    OfficialSource {
        name: "Info.gouv.fr",
        search_url: "https://www.info.gouv.fr/recherche",
        query_param: "q",
        content_selectors: &[".article-content", ".content-body", "main article"],
        allowed_domains: &["info.gouv.fr"],
        categories: &["general"],
    },
    OfficialSource {
        name: "Data.gouv.fr",
        search_url: "https://www.data.gouv.fr/datasets/search",
        query_param: "q",
        content_selectors: &[".dataset-description", ".content", "main article"],
        allowed_domains: &["data.gouv.fr"],
        categories: &["general", "statistiques"],
    },
    // ─── Institutions & Vie politique ──────────────────────────────────────────
    OfficialSource {
        name: "Assemblée nationale",
        search_url: "https://www2.assemblee-nationale.fr/recherche/resultats_recherche",
        query_param: "unk",
        content_selectors: &[".article-content", ".bloc-content", "main article"],
        allowed_domains: &["assemblee-nationale.fr"],
        categories: &["institutions", "droit_travail", "general"],
    },
    OfficialSource {
        name: "Sénat",
        search_url: "https://www.senat.fr/basile/rechercheGlobale.do",
        query_param: "unk",
        content_selectors: &[".article-content", ".text-content", "main article"],
        allowed_domains: &["senat.fr"],
        categories: &["institutions", "general"],
    },
    OfficialSource {
        name: "Annuaire de l'Administration",
        search_url: "https://lannuaire.service-public.gouv.fr/recherche",
        query_param: "whoWhat",
        content_selectors: &[".content-body", ".field--name-body", "main article"],
        allowed_domains: &["lannuaire.service-public.gouv.fr", "lannuaire.service-public.fr"],
        categories: &["general", "institutions"],
    },
    OfficialSource {
        name: "Vie-publique.fr",
        search_url: "https://www.vie-publique.fr/recherche",
        query_param: "search_api_fulltext",
        content_selectors: &[".field--name-body", ".article-content", "main article"],
        allowed_domains: &["vie-publique.fr"],
        categories: &["general", "institutions", "droit_travail", "logement"],
    },
    // ─── Économie, Banque et Finance ───────────────────────────────────────────
    OfficialSource {
        name: "Economie.gouv.fr",
        search_url: "https://www.economie.gouv.fr/recherche-resultat",
        query_param: "search_api_views_fulltext",
        content_selectors: &[".field--name-body", ".content-area", "main article"],
        allowed_domains: &["economie.gouv.fr"],
        categories: &["impots", "fiscalite", "consommation", "autoentrepreneur"],
    },
    OfficialSource {
        name: "Banque de France",
        search_url: "https://www.banque-france.fr/fr/recherche",
        query_param: "search_api_fulltext",
        content_selectors: &[".field--name-body", ".content", "main article"],
        allowed_domains: &["banque-france.fr"],
        categories: &["fiscalite", "surendettement"],
    },
    OfficialSource {
        name: "La finance pour tous",
        search_url: "https://www.lafinancepourtous.com/",
        query_param: "s",
        content_selectors: &[".entry-content", ".article-content", "main article"],
        allowed_domains: &["lafinancepourtous.com"],
        categories: &["fiscalite", "general"],
    },
    OfficialSource {
        name: "AMF",
        search_url: "https://www.amf-france.org/fr/recherche/resultat",
        query_param: "key",
        content_selectors: &[".field--name-body", ".content", "main article"],
        allowed_domains: &["amf-france.org"],
        categories: &["fiscalite", "consommation"],
    },
    // ─── Statistiques & Journal Officiel ───────────────────────────────────────
    OfficialSource {
        name: "INSEE",
        search_url: "https://www.insee.fr/fr/recherche",
        query_param: "q",
        content_selectors: &[".article-content", ".content", "main article"],
        allowed_domains: &["insee.fr"],
        categories: &["statistiques", "general"],
    },
];

/// Sélectionner les sources pertinentes selon la catégorie
pub fn select_sources(category: &str) -> Vec<&'static OfficialSource> {
    OFFICIAL_SOURCES
        .iter()
        .filter(|s| s.categories.contains(&category) || s.categories.contains(&"general"))
        .collect()
}
