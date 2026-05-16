use super::sources::{select_sources, OfficialSource};
use anyhow::Result;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
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
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(8))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()?;
        Ok(Self { client })
    }

    /// Rechercher des informations — sources officielles en priorité, puis web général
    pub async fn search(
        &self,
        query: &str,
        category: &str,
        max_results: usize,
    ) -> Result<Vec<WebResult>> {
        let mut all_results = Vec::new();
        let mut seen_domains = HashSet::new();

        // Phase 1 : Sources officielles (prioritaires)
        let official_results = self.search_official(query, category).await;
        for result in official_results {
            let domain = extract_result_domain(&result.url);
            if seen_domains.insert(domain) {
                all_results.push(result);
            }
        }

        tracing::info!(
            "Phase 1 (sources officielles) : {} résultat(s) pour '{}'",
            all_results.len(),
            &query[..50.min(query.len())]
        );

        // Phase 2 : Si pas assez de résultats, recherche web générale via DuckDuckGo
        if all_results.len() < max_results {
            let remaining = max_results - all_results.len();
            let general_results = self.search_general(query, remaining, &seen_domains).await;
            for result in general_results {
                let domain = extract_result_domain(&result.url);
                if seen_domains.insert(domain) {
                    all_results.push(result);
                    if all_results.len() >= max_results {
                        break;
                    }
                }
            }
            tracing::info!(
                "Phase 2 (web général) : total {} résultat(s)",
                all_results.len()
            );
        }

        Ok(all_results.into_iter().take(max_results).collect())
    }

    /// Phase 1 : Recherche sur les sources officielles françaises
    async fn search_official(&self, query: &str, category: &str) -> Vec<WebResult> {
        let sources = select_sources(category);
        let mut results = Vec::new();

        let search_fut = async {
            for source in sources.into_iter().take(3) {
                match search_one_source(&self.client, source, query).await {
                    Ok(r) => results.extend(r),
                    Err(e) => tracing::debug!("Source {} échouée : {}", source.name, e),
                }
            }
        };

        match tokio::time::timeout(Duration::from_secs(10), search_fut).await {
            Ok(()) => {}
            Err(_) => tracing::warn!("Timeout sources officielles — résultats partiels"),
        }

        results
    }

    /// Phase 2 : Recherche web générale via DuckDuckGo HTML (pas d'API key requise)
    async fn search_general(
        &self,
        query: &str,
        max_results: usize,
        exclude_domains: &HashSet<String>,
    ) -> Vec<WebResult> {
        let mut results = Vec::new();

        let search_fut = async {
            // Ajouter "france" et "droit" pour orienter les résultats
            let enriched_query = format!("{} france droit", query);
            match self.search_duckduckgo(&enriched_query, exclude_domains).await {
                Ok(r) => results.extend(r),
                Err(e) => tracing::debug!("DuckDuckGo échoué : {}", e),
            }
        };

        match tokio::time::timeout(Duration::from_secs(10), search_fut).await {
            Ok(()) => {}
            Err(_) => tracing::warn!("Timeout recherche web générale"),
        }

        results.into_iter().take(max_results).collect()
    }

    /// Interroger DuckDuckGo HTML et extraire les résultats
    async fn search_duckduckgo(
        &self,
        query: &str,
        exclude_domains: &HashSet<String>,
    ) -> Result<Vec<WebResult>> {
        let url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self.client
            .get(&url)
            .header("Accept-Language", "fr-FR,fr;q=0.9")
            .send()
            .await?;

        let html = response.text().await?;
        let links = extract_duckduckgo_links(&html, exclude_domains);

        let mut results = Vec::new();
        for (page_url, title) in links.into_iter().take(5) {
            match self.fetch_general_page(&page_url).await {
                Ok(content) if content.len() > 150 && is_coherent_text(&content) => {
                    let snippet = content.chars().take(250).collect::<String>();
                    let source_name = extract_result_domain(&page_url);
                    results.push(WebResult {
                        title,
                        url: page_url,
                        source_name,
                        content,
                        snippet,
                    });
                }
                Ok(content) => {
                    tracing::debug!("Contenu rejeté (incohérent ou trop court) : {} chars", content.len());
                }
                _ => {}
            }
            if results.len() >= 3 {
                break;
            }
        }

        Ok(results)
    }

    /// Fetch et extraire le contenu d'une page web générale
    async fn fetch_general_page(&self, url: &str) -> Result<String> {
        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        let html = response.text().await?;
        let content = extract_general_content(&html);

        if content.len() < 100 {
            anyhow::bail!("Contenu insuffisant");
        }

        Ok(sanitize_web_content(&content).chars().take(2000).collect())
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
            if content.len() > 100 && is_coherent_text(&content) {
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

/// Extraire le domaine d'une URL (pour déduplication et affichage)
fn extract_result_domain(url: &str) -> String {
    url.split("//")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or(url)
        .replace("www.", "")
        .replace("www2.", "")
        .to_lowercase()
}

/// Extraire les liens de résultats DuckDuckGo HTML
fn extract_duckduckgo_links(html: &str, exclude_domains: &HashSet<String>) -> Vec<(String, String)> {
    let document = Html::parse_document(html);
    let mut links = Vec::new();
    let mut seen_domains = HashSet::new();

    // DuckDuckGo HTML : les résultats sont dans .result__a
    let selectors = [
        ".result__a",
        ".results_links_deep a.result__url",
        "a.result__a",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector).take(10) {
                let href = element.value().attr("href").unwrap_or("");
                let title = element.text().collect::<String>().trim().to_string();

                if href.is_empty() || title.len() < 5 {
                    continue;
                }

                // DuckDuckGo encode les URLs dans un redirect — extraire l'URL réelle
                let real_url = if href.contains("uddg=") {
                    href.split("uddg=")
                        .nth(1)
                        .and_then(|u| urlencoding::decode(u.split('&').next().unwrap_or(u)).ok())
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| href.to_string())
                } else if href.starts_with("http") {
                    href.to_string()
                } else {
                    continue;
                };

                // Filtrer les domaines déjà couverts par les sources officielles
                let domain = extract_result_domain(&real_url);
                if exclude_domains.contains(&domain) {
                    continue;
                }

                // Exclure les sites non pertinents
                let blocked_domains = [
                    "youtube.com", "facebook.com", "twitter.com", "x.com",
                    "instagram.com", "tiktok.com", "reddit.com", "pinterest.com",
                    "amazon.fr", "amazon.com", "ebay.fr", "leboncoin.fr",
                    "wikipedia.org",
                    "duckduckgo.com", // Ne pas inclure DDG lui-même
                    // Écoles / formations privées (contenu marketing, pas informatif)
                    "jedha.com", "openclassrooms.com", "udemy.com", "coursera.org",
                    "datascientest.com", "wildcodeschool.com",
                ];
                if blocked_domains.iter().any(|d| domain.contains(d)) {
                    continue;
                }

                if seen_domains.insert(domain) {
                    links.push((real_url, title));
                }
            }
        }
        if !links.is_empty() {
            break;
        }
    }

    links
}

/// Extraire le contenu principal d'une page web générale (non officielle)
fn extract_general_content(html: &str) -> String {
    let document = Html::parse_document(html);

    // Sélecteurs prioritaires pour le contenu éditorial
    let content_selectors = [
        "article .entry-content",
        "article .post-content",
        ".article-body",
        ".article-content",
        ".post-content",
        ".entry-content",
        "[itemprop=articleBody]",
        "main article",
        "main .content",
        "main",
        "article",
        "#content",
        ".content",
    ];

    for sel_str in &content_selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            let texts: Vec<String> = document
                .select(&selector)
                .flat_map(|el| el.text().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()))
                .collect();

            if texts.len() > 3 {
                let content = texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
                if is_quality_content(&content) {
                    return content;
                }
            }
        }
    }

    String::new()
}

/// Vérifier si un texte est cohérent et lisible (pas du garbage/gibberish)
/// Détecte les contenus corrompus, les pages anti-bot, et le bruit web
fn is_coherent_text(text: &str) -> bool {
    if text.len() < 100 {
        return false;
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let word_count = words.len();

    if word_count < 20 {
        return false;
    }

    // Ratio de mots très courts (1-2 chars) : du gibberish a beaucoup de fragments
    let very_short_words = words.iter().filter(|w| w.chars().count() <= 2).count();
    if very_short_words as f32 / word_count as f32 > 0.5 {
        return false;
    }

    // Ratio de mots très longs (>25 chars) : signe de concaténation/corruption
    let very_long_words = words.iter().filter(|w| w.chars().count() > 25).count();
    if very_long_words as f32 / word_count as f32 > 0.1 {
        return false;
    }

    // Caractères non-alphabétiques excessifs (hors ponctuation normale)
    let total_chars = text.chars().count();
    let garbage_chars = text.chars().filter(|c| {
        !c.is_alphanumeric() && !c.is_whitespace()
            && !matches!(*c, '.' | ',' | ';' | ':' | '!' | '?' | '-' | '\'' | '"' | '(' | ')' | '/' | 'é' | 'è' | 'ê' | 'à' | 'â' | 'ù' | 'û' | 'ô' | 'î' | 'ï' | 'ç' | 'œ' | 'æ' | '«' | '»' | '€' | '°' | '—' | '–')
    }).count();
    if total_chars > 0 && garbage_chars as f32 / total_chars as f32 > 0.15 {
        return false;
    }

    // Vérifier qu'il y a des phrases (au moins quelques points ou retours à la ligne)
    let sentence_endings = text.chars().filter(|c| matches!(*c, '.' | '!' | '?')).count();
    if sentence_endings == 0 && word_count > 50 {
        return false;
    }

    // Rejeter si le texte contient des marqueurs d'erreur anti-bot
    let lower = text.to_lowercase();
    let antibot_markers = [
        "captcha", "robot", "access denied", "403 forbidden",
        "enable javascript", "please enable cookies",
        "checking your browser", "cloudflare", "just a moment",
        "veuillez activer javascript",
    ];
    if antibot_markers.iter().any(|m| lower.contains(m)) {
        return false;
    }

    // Rejeter si le texte est principalement du code HTML/JS résiduel
    let code_markers = ["function(", "var ", "const ", "{return", "};", "onclick=", "href=\"#\""];
    let code_hits = code_markers.iter().filter(|m| text.contains(*m)).count();
    if code_hits >= 3 {
        return false;
    }

    true
}
