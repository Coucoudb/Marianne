use super::sources::{select_sources, OfficialSource};
use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

/// Résultat d'une page web extraite
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WebResult {
    pub title: String,
    pub url: String,
    pub source_name: String,
    pub content: String,
    pub snippet: String,
}

pub struct WebSearcher {
    client: Client,
}

impl WebSearcher {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; MarianneBot/1.0)")
            .timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()?;
        Ok(Self { client })
    }

    /// Rechercher sur les sources officielles
    pub async fn search(
        &self,
        query: &str,
        category: &str,
        max_results: usize,
    ) -> Result<Vec<WebResult>> {
        let sources = select_sources(category);
        let mut all_results = Vec::new();

        let search_fut = async {
            for source in sources.into_iter().take(3) {
                match search_one_source(&self.client, source, query).await {
                    Ok(results) => {
                        all_results.extend(results);
                        if all_results.len() >= max_results {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Source {} échouée : {}", source.name, e);
                    }
                }
            }
        };

        // Timeout global de 10s pour toutes les sources
        match tokio::time::timeout(Duration::from_secs(10), search_fut).await {
            Ok(()) => {}
            Err(_) => {
                tracing::warn!("Timeout recherche web — résultats partiels");
            }
        }

        tracing::info!("Recherche web '{}' → {} résultats", query, all_results.len());
        Ok(all_results.into_iter().take(max_results).collect())
    }
}

async fn search_one_source(
    client: &Client,
    source: &OfficialSource,
    query: &str,
) -> Result<Vec<WebResult>> {
    let search_url = if source.query_param.is_empty() {
        // Recherche par chemin (ex: justice.fr/recherche/all/terme)
        format!("{}/{}", source.search_url, urlencoding::encode(query))
    } else {
        format!(
            "{}?{}={}",
            source.search_url,
            source.query_param,
            urlencoding::encode(query)
        )
    };

    let response = client.get(&search_url).send().await?;
    let html = response.text().await?;

    // Extraire les liens dans un bloc séparé pour dropper `document` avant les .await
    let links = extract_links(&html, source);

    // Maintenant fetch les pages (safe, document droppé)
    let mut results = Vec::new();
    for (full_url, title) in links {
        if let Ok(content) = fetch_and_extract(client, &full_url, source).await {
            if content.len() > 100 {
                let snippet = content.chars().take(250).collect::<String>();
                results.push(WebResult {
                    title,
                    url: full_url,
                    source_name: source.name.to_string(),
                    content,
                    snippet,
                });
            }
        }

        if results.len() >= 2 {
            break;
        }
    }

    Ok(results)
}

/// Extraire les liens d'une page de résultats de recherche (sync, pas de .await)
fn extract_links(html: &str, source: &OfficialSource) -> Vec<(String, String)> {
    let document = Html::parse_document(html);
    let mut links: Vec<(String, String)> = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();

    // Groupes de sélecteurs par priorité — on s'arrête au premier groupe qui donne des résultats
    let selector_groups: &[&[&str]] = &[
        // 1. DSFR (Design Système de l'État français) — sites modernes *.gouv.fr
        &[
            ".fr-card__title a",
            ".fr-card a[href]",
            ".fr-card__link",
            ".fr-tile__link",
            ".fr-tile__title a",
        ],
        // 2. Drupal / CMS gouvernementaux
        &[
            ".views-row a[href]",
            ".view-content a[href]",
            ".node--type-article a[href]",
            ".field--name-title a",
        ],
        // 3. Patterns classiques de résultats de recherche
        &[
            ".search-result a",
            ".search-results a",
            ".result-item a",
            ".results-list a",
            "li.result a[href]",
            ".search-result-link",
            ".result a[href]",
        ],
        // 4. Liens dans des titres (très courant pour les listes de résultats)
        &["h2 a[href]", "h3 a[href]"],
        // 5. Liens dans le contenu principal
        &["article a[href]", "main li a[href]", "#content li a[href]"],
    ];

    for group in selector_groups {
        for selector_str in *group {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector).take(5) {
                    let href = element.value().attr("href").unwrap_or("");
                    let title = element.text().collect::<String>().trim().to_string();

                    // Filtrer titres trop courts (nav, icônes) et liens vides
                    if href.is_empty() || title.len() < 10 {
                        continue;
                    }

                    let full_url = if href.starts_with("http") {
                        href.to_string()
                    } else if href.starts_with('/') {
                        format!("https://{}{}", source.allowed_domains[0], href)
                    } else {
                        format!("https://{}/{}", source.allowed_domains[0], href)
                    };

                    if !source.allowed_domains.iter().any(|d| full_url.contains(d)) {
                        continue;
                    }

                    if seen_urls.insert(full_url.clone()) {
                        links.push((full_url, title));
                    }
                }
            }
        }
        if !links.is_empty() {
            tracing::debug!(
                "Source {} : {} liens trouvés (groupe {:?})",
                source.name,
                links.len(),
                group.first().unwrap_or(&"?")
            );
            break;
        }
    }

    if links.is_empty() {
        tracing::debug!("Source {} : aucun lien extrait de la page de recherche", source.name);
    }

    links
}

async fn fetch_and_extract(
    client: &Client,
    url: &str,
    source: &OfficialSource,
) -> Result<String> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;

    // Vérifier que le domaine est autorisé après redirection
    let response_host = response.url().host_str().unwrap_or("");
    if !source
        .allowed_domains
        .iter()
        .any(|d| response_host.ends_with(d))
    {
        anyhow::bail!("Domaine non autorisé après redirection : {}", response_host);
    }

    let html = response.text().await?;
    let extracted = extract_text(&html, source.content_selectors);

    if extracted.is_empty() {
        anyhow::bail!("Aucun contenu utile extrait de {}", url);
    }

    let cleaned = sanitize_web_content(&extracted);
    if cleaned.len() < 100 {
        anyhow::bail!("Contenu trop court après nettoyage de {}", url);
    }

    Ok(cleaned.chars().take(2000).collect())
}

/// Extraire le texte brut d'un HTML selon des sélecteurs CSS
fn extract_text(html: &str, selectors: &[&str]) -> String {
    let document = Html::parse_document(html);

    // D'abord supprimer le contenu de navigation/menu/header/footer du texte
    // en collectant seulement depuis les zones de contenu
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            let texts: Vec<String> = document
                .select(&selector)
                .flat_map(|el| el.text().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()))
                .collect();

            if !texts.is_empty() {
                let content = texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
                if is_quality_content(&content) {
                    return content;
                }
            }
        }
    }

    // Fallback amélioré : chercher <main> ou <article> en excluant nav/header/footer
    let fallback_selectors = [
        "main article",
        "main",
        "article",
        "#content",
        ".content",
        "[role=main]",
    ];

    for sel_str in &fallback_selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            let texts: Vec<String> = document
                .select(&selector)
                .flat_map(|el| el.text().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()))
                .collect();

            if texts.len() > 5 {
                let content = texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
                if is_quality_content(&content) {
                    return content;
                }
            }
        }
    }

    // Dernier recours : retourner vide plutôt que du contenu de navigation
    tracing::debug!("Aucun contenu utile extrait de la page");
    String::new()
}

/// Vérifier que le contenu extrait est du vrai contenu éditorial
/// et non du bruit de navigation (menus, headers, footers).
fn is_quality_content(text: &str) -> bool {
    // Trop court = pas de contenu utile
    if text.len() < 100 {
        return false;
    }

    let lower = text.to_lowercase();

    // Compter les marqueurs de navigation
    let nav_markers = [
        "menu", "recherche", "rechercher", "se connecter", "connexion",
        "accueil", "pied de page", "fermer", "ouvrir la recherche",
        "fiches pratiques", "événement de vie", "voir tous",
        "cookie", "accepter", "refuser", "paramétrer",
        "république française", "var marquage",
    ];
    let nav_count = nav_markers.iter().filter(|m| lower.contains(*m)).count();

    // Si plus de 5 marqueurs de nav sur un texte court, c'est du bruit
    if nav_count > 5 && text.len() < 500 {
        return false;
    }

    // Ratio : si >30% du texte est composé de mots de navigation, c'est suspect
    let nav_words: usize = nav_markers
        .iter()
        .map(|m| lower.matches(m).count())
        .sum();
    let total_words = text.split_whitespace().count();
    if total_words > 0 && nav_words as f32 / total_words as f32 > 0.15 {
        return false;
    }

    true
}

/// Nettoyer le contenu web extrait des artefacts de scraping.
fn sanitize_web_content(text: &str) -> String {
    let mut cleaned = text.to_string();

    // Supprimer les artefacts de templates Mustache/Handlebars : {{{...}}}, {{...}}
    let re_triple = regex_lite::Regex::new(r"\{\{\{[^}]*\}?\}?\}?").unwrap();
    cleaned = re_triple.replace_all(&cleaned, "").to_string();
    let re_double = regex_lite::Regex::new(r"\{\{[^}]*\}?\}?").unwrap();
    cleaned = re_double.replace_all(&cleaned, "").to_string();

    // Supprimer les messages de JS désactivé et autres bruits communs
    let noise_patterns = [
        "Javascript est désactivé dans votre navigateur",
        "JavaScript est désactivé",
        "Vous devez activer JavaScript",
        "Ce site nécessite JavaScript",
        "var marquage_authentication",
        "var marquage",
        "Ouvrir la recherche",
        "Fermer la recherche",
        "Aller au contenu",
        "Aller au menu",
        "skip to content",
    ];
    for pattern in &noise_patterns {
        cleaned = cleaned.replace(pattern, "");
    }

    // Nettoyer les espaces multiples et lignes vides
    cleaned = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    cleaned
}
