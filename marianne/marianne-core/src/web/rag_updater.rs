use dashmap::DashSet;

/// Calculer le hash d'un contenu textuel pour déduplication
pub fn content_hash(text: &str) -> String {
    let hash = xxhash_rust::xxh3::xxh3_64(text.as_bytes());
    format!("{:x}", hash)
}

/// Vérifie si un hash est déjà connu — O(1) en mémoire
pub fn is_known_hash(hash: &str, known_hashes: &DashSet<String>) -> bool {
    known_hashes.contains(hash)
}

/// Après ingestion réussie : enregistrer le hash pour les futures déduplications
pub fn register_hash(hash: &str, known_hashes: &DashSet<String>) {
    known_hashes.insert(hash.to_string());
}

/// Vérifier si un contenu est un doublon et l'enregistrer sinon
/// Retourne true si c'est un doublon (déjà connu), false si nouveau
pub fn check_and_register(text: &str, known_hashes: &DashSet<String>) -> bool {
    let hash = content_hash(text);
    if is_known_hash(&hash, known_hashes) {
        true // doublon
    } else {
        register_hash(&hash, known_hashes);
        false // nouveau
    }
}
