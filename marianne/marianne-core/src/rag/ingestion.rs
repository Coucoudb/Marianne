// src-tauri/src/rag/ingestion.rs
use super::{
    embedder::{embed_passages, init_embedder},
    store::{KnowledgeChunk, VectorStore},
};
use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

/// Découper un texte en chunks de taille optimale avec heuristiques sémantiques.
/// Respecte les frontières de paragraphes, de titres, et maintient le contexte.
pub fn semantic_chunk(text: &str, max_chars: usize) -> Vec<String> {
    let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_section_title = String::new();

    for paragraph in paragraphs {
        let trimmed_para = paragraph.trim();
        
        // Détecter un titre de section
        if trimmed_para.starts_with('#') {
            if let Some(title) = trimmed_para.lines().next() {
                current_section_title = title.to_string();
            }
            
            // Forcer une coupure de chunk avant une nouvelle section si le chunk actuel est assez grand
            if current_chunk.len() > 200 {
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
            }
        }

        // Si le paragraphe est lui-même plus grand que le max (rare mais possible)
        if trimmed_para.len() > max_chars {
            let sentences: Vec<&str> = trimmed_para
                .split(['.', '!', '?'])
                .filter(|s| !s.trim().is_empty())
                .collect();

            for sentence in sentences {
                if current_chunk.len() + sentence.len() > max_chars && !current_chunk.is_empty() {
                    chunks.push(current_chunk.trim().to_string());
                    
                    // Overlap: garder la dernière phrase comme contexte
                    let last_sentence = current_chunk.split(". ").last().unwrap_or("").to_string();
                    current_chunk = format!("{} [Suite de {}] ... {}", last_sentence, current_section_title, sentence);
                } else {
                    current_chunk.push_str(sentence);
                    current_chunk.push_str(". ");
                }
            }
        } else {
            if current_chunk.len() + trimmed_para.len() > max_chars && !current_chunk.is_empty() {
                chunks.push(current_chunk.trim().to_string());
                
                // Overlap: garder le dernier paragraphe du chunk précédent
                let last_para = current_chunk.split("\n\n").last().unwrap_or("").to_string();
                
                // Commencer le nouveau chunk avec le titre de section et l'overlap
                current_chunk = String::new();
                if !current_section_title.is_empty() {
                    current_chunk.push_str(&format!("Contexte : {}\n\n", current_section_title));
                }
                current_chunk.push_str(&last_para);
                current_chunk.push_str("\n\n");
            }
            
            // Si on commence un nouveau chunk, inclure le titre
            if current_chunk.is_empty() && !current_section_title.is_empty() && !trimmed_para.starts_with('#') {
                current_chunk.push_str(&format!("Contexte : {}\n\n", current_section_title));
            }
            
            current_chunk.push_str(trimmed_para);
            current_chunk.push_str("\n\n");
        }
    }

    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    // Fusionner les micro-chunks (< 200 chars) avec le précédent s'il y en a un
    let mut final_chunks: Vec<String> = Vec::new();
    for chunk in chunks {
        if chunk.len() < 200 && !final_chunks.is_empty() {
            let mut last = final_chunks.pop().unwrap();
            last.push_str("\n\n");
            last.push_str(&chunk);
            final_chunks.push(last);
        } else {
            final_chunks.push(chunk);
        }
    }

    final_chunks
}

/// Descriptions textuelles des catégories pour le matching sémantique.
/// Pour ajouter une catégorie, il suffit d'ajouter une entrée (tag, description).
/// L'embedding de chaque description est calculé une seule fois au premier appel.
const CATEGORY_DESCRIPTIONS: &[(&str, &str)] = &[
    // ── Droits & Administration ──────────────────────────────────────────
    ("caf", "Caisse d'Allocations Familiales, APL, RSA, prime d'activité, allocations sociales, aide au logement"),
    ("urssaf", "URSSAF, auto-entrepreneur, micro-entreprise, cotisations sociales, charges patronales"),
    ("sante", "Sécurité sociale, Ameli, CPAM, assurance maladie, mutuelle, arrêt maladie, médecin, hôpital"),
    ("droit_travail", "Droit du travail, contrat de travail, licenciement, démission, CDD, CDI, prud'hommes, heures supplémentaires"),
    ("logement", "Logement, bail, locataire, propriétaire, loyer, expulsion, préavis, état des lieux, dépôt de garantie"),
    ("retraite", "Retraite, pension, trimestres, CNAV, Agirc-Arrco, âge de départ, surcote, décote"),
    ("recours", "Recours administratif, contestation, litige, tribunal, médiation, procédure contentieuse, délai de recours"),
    ("impots", "Impôts, fiscalité, déclaration de revenus, taxe foncière, taxe d'habitation, crédit d'impôt, TVA"),
    // ── Droit français étendu ────────────────────────────────────────────
    ("procedure_judiciaire", "Procédure judiciaire, saisine du tribunal, assignation, requête, audience, jugement, appel, cassation"),
    ("code_civil", "Code civil, obligation, contrat, responsabilité civile, dommages et intérêts, prescription"),
    ("consommation", "Droit de la consommation, garantie légale, droit de rétractation, pratiques commerciales trompeuses, litige vendeur"),
    ("penal", "Droit pénal, infraction, délit, contravention, crime, plainte, garde à vue, procureur"),
    ("famille", "Divorce, pension alimentaire, garde d'enfant, autorité parentale, PACS, mariage, adoption, succession"),
    ("jurisprudence", "Jurisprudence, arrêt de la Cour de cassation, décision du Conseil d'État, précédent judiciaire"),
    ("droit_immobilier", "Copropriété, permis de construire, urbanisme, servitude, cadastre, vente immobilière, notaire"),
    ("droit_numerique", "RGPD, données personnelles, CNIL, droit à l'oubli, cookies, vie privée numérique, cyberharcèlement"),
    // ── Finance & Économie ───────────────────────────────────────────────
    ("finance_perso", "Budget personnel, épargne, crédit bancaire, prêt immobilier, taux d'intérêt, surendettement, Banque de France"),
    ("investissement", "Bourse, actions, obligations, ETF, dividendes, plus-value, PEA, assurance-vie, placement financier"),
    ("crypto", "Cryptomonnaie, Bitcoin, Ethereum, blockchain, wallet, DeFi, NFT, minage, token"),
    ("comptabilite", "Comptabilité, bilan, compte de résultat, amortissement, TVA, facture, liasse fiscale"),
    // ── Vie quotidienne ──────────────────────────────────────────────────
    ("transport", "Transport, permis de conduire, carte grise, contravention routière, assurance auto, covoiturage, SNCF"),
    ("education", "École, université, inscription, bourse étudiante, diplôme, formation professionnelle, CPF, apprentissage"),
    ("emploi", "Recherche d'emploi, CV, lettre de motivation, entretien d'embauche, France Travail, chômage, ARE, création d'entreprise"),
    ("demarches", "Démarches administratives, carte d'identité, passeport, acte de naissance, mairie, préfecture, état civil"),
    ("consommateur", "Droits du consommateur, réclamation, garantie, service après-vente, arnaque, association de consommateurs"),
    ("environnement", "Écologie, recyclage, énergie renouvelable, isolation thermique, MaPrimeRénov, diagnostic énergétique"),
    // ── Programmation & Tech ─────────────────────────────────────────────
    ("programmation", "Programmation, code source, développement logiciel, IDE, debug, compilateur, langage de programmation"),
    ("algorithmique", "Algorithme, structure de données, complexité, tri, recherche, graphe, arbre, programmation dynamique, récursion"),
    ("web_dev", "Développement web, HTML, CSS, JavaScript, React, Svelte, Angular, API REST, frontend, backend, Node.js"),
    ("rust", "Langage Rust, ownership, borrowing, lifetime, cargo, crate, trait, unsafe, async Rust, tokio"),
    ("python", "Python, pip, pandas, numpy, Django, Flask, machine learning, scripting, data science"),
    ("devops", "DevOps, Docker, Kubernetes, CI/CD, GitHub Actions, déploiement, infrastructure, monitoring, Terraform"),
    ("base_donnees", "Base de données, SQL, PostgreSQL, MongoDB, requête, index, migration, ORM, NoSQL, Redis"),
    ("ia_ml", "Intelligence artificielle, machine learning, deep learning, réseau de neurones, LLM, GPT, entraînement, fine-tuning, RAG"),
    // ── Cybersécurité & Hacking ──────────────────────────────────────────
    ("cybersecurite", "Cybersécurité, sécurité informatique, pare-feu, antivirus, chiffrement, authentification, zero trust, SOC"),
    ("hacking", "Hacking éthique, pentest, test d'intrusion, vulnérabilité, exploit, CTF, reverse engineering, bug bounty"),
    ("reseau", "Réseau informatique, TCP/IP, DNS, VPN, proxy, routeur, switch, Wireshark, pare-feu, NAT"),
    ("crypto_secu", "Cryptographie, chiffrement AES, RSA, hachage SHA, certificat SSL/TLS, PKI, signature numérique"),
    // ── Sciences & Culture ───────────────────────────────────────────────
    ("histoire", "Histoire, événement historique, guerre, révolution, civilisation, monarchie, république, chronologie"),
    ("geographie", "Géographie, pays, continent, capitale, population, climat, cartographie, territoire"),
    ("sciences", "Sciences, physique, chimie, biologie, mathématiques, expérience, théorie, formule, recherche scientifique"),
    ("philosophie", "Philosophie, éthique, morale, logique, métaphysique, penseur, courant philosophique, existentialisme"),
    ("litterature", "Littérature, roman, poésie, théâtre, auteur, analyse littéraire, courant littéraire, prix littéraire"),
    ("culture_generale", "Culture générale, art, musique, cinéma, architecture, musée, patrimoine, actualité"),
    // ── Catch-all ────────────────────────────────────────────────────────
    ("general", "Question générale, information diverse, aide, conseil, explication, définition"),
];

/// Embeddings pré-calculés des descriptions de catégories (lazy init).
/// Calculés une seule fois au premier appel, puis réutilisés.
use once_cell::sync::OnceCell;
use std::sync::Mutex as StdMutex;

static CATEGORY_EMBEDDINGS: OnceCell<StdMutex<Vec<(String, Vec<f32>)>>> = OnceCell::new();

/// Initialiser les embeddings des catégories (une seule fois)
fn ensure_category_embeddings() -> anyhow::Result<()> {
    if CATEGORY_EMBEDDINGS.get().is_some() {
        return Ok(());
    }

    let descriptions: Vec<&str> = CATEGORY_DESCRIPTIONS.iter().map(|(_, desc)| *desc).collect();
    let embeddings = embed_passages(&descriptions)?;

    let category_embeddings: Vec<(String, Vec<f32>)> = CATEGORY_DESCRIPTIONS
        .iter()
        .zip(embeddings.into_iter())
        .map(|((tag, _), emb)| (tag.to_string(), emb))
        .collect();

    let _ = CATEGORY_EMBEDDINGS.set(StdMutex::new(category_embeddings));
    tracing::info!("✅ {} catégories initialisées pour la catégorisation sémantique", CATEGORY_DESCRIPTIONS.len());
    Ok(())
}

/// Catégoriser un chunk par similarité sémantique avec les descriptions de catégories.
/// Retourne les top-K catégories dont la similarité dépasse le seuil.
fn semantic_categorize(chunk_embedding: &[f32]) -> String {
    const SIMILARITY_THRESHOLD: f32 = 0.35;
    const MAX_TAGS: usize = 3;

    let guard = match CATEGORY_EMBEDDINGS.get() {
        Some(m) => match m.lock() {
            Ok(g) => g,
            Err(_) => return "[\"general\"]".to_string(),
        },
        None => return "[\"general\"]".to_string(),
    };

    let mut scores: Vec<(&str, f32)> = guard
        .iter()
        .map(|(tag, cat_emb)| {
            let sim = cosine_similarity(chunk_embedding, cat_emb);
            (tag.as_str(), sim)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let tags: Vec<&str> = scores
        .iter()
        .filter(|(_, sim)| *sim >= SIMILARITY_THRESHOLD)
        .take(MAX_TAGS)
        .map(|(tag, _)| *tag)
        .collect();

    if tags.is_empty() {
        "[\"general\"]".to_string()
    } else {
        serde_json::to_string(&tags).unwrap_or_else(|_| "[\"general\"]".to_string())
    }
}

/// Similarité cosine entre deux vecteurs
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot / (norm_a * norm_b)
}

/// Ingérer tous les fichiers Markdown du corpus
pub async fn ingest_corpus(
    corpus_dir: &Path,
    store: &VectorStore,
    models_dir: &Path,
) -> Result<usize> {
    init_embedder(models_dir)?;
    // Pré-calculer les embeddings des catégories (une seule fois)
    ensure_category_embeddings()?;
    store.ensure_table().await?;

    let mut total_chunks = 0;

    let mut entries = tokio::fs::read_dir(corpus_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let filename = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = tokio::fs::read_to_string(&path).await?;
        let raw_chunks = semantic_chunk(&content, 800);
        let chunk_texts: Vec<&str> = raw_chunks.iter().map(|s| s.as_str()).collect();

        tracing::info!("Ingestion de {} : {} chunks", filename, raw_chunks.len());

        for batch in chunk_texts.chunks(32) {
            let embeddings = embed_passages(batch)?;

            let knowledge_chunks: Vec<KnowledgeChunk> = batch
                .iter()
                .zip(embeddings.iter())
                .map(|(text, embedding)| {
                    // Catégorisation sémantique : réutilise l'embedding déjà calculé
                    KnowledgeChunk {
                        id: Uuid::new_v4().to_string(),
                        text: text.to_string(),
                        source: filename.clone(),
                        tags: semantic_categorize(embedding),
                        embedding: embedding.clone(),
                    }
                })
                .collect();

            total_chunks += store.insert_chunks(&knowledge_chunks).await?;
        }
    }

    tracing::info!("✅ Corpus ingéré : {} chunks total", total_chunks);
    Ok(total_chunks)
}
