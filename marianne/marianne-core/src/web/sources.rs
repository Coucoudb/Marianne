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

pub const OFFICIAL_SOURCES: &[OfficialSource] = &[];

/// Sélectionner les sources pertinentes selon la catégorie
pub fn select_sources(category: &str) -> Vec<&'static OfficialSource> {
    OFFICIAL_SOURCES
        .iter()
        .filter(|s| s.categories.contains(&category) || s.categories.contains(&"general"))
        .collect()
}
