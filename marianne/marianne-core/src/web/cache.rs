use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use anyhow::Result;

/// Durée de validité du cache web (24h)
const CACHE_TTL: Duration = Duration::from_secs(24 * 3600);

pub struct WebCache {
    cache_dir: PathBuf,
}

impl WebCache {
    pub fn new(cache_dir: &Path) -> Self {
        std::fs::create_dir_all(cache_dir).ok();
        Self {
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    fn cache_key(&self, query: &str, category: &str) -> PathBuf {
        let hash = xxhash_rust::xxh3::xxh3_64(format!("{}:{}", query, category).as_bytes());
        self.cache_dir.join(format!("{:x}.json", hash))
    }

    /// Récupérer depuis le cache si valide
    pub fn get(&self, query: &str, category: &str) -> Option<Vec<super::searcher::WebResult>> {
        let path = self.cache_key(query, category);
        if !path.exists() {
            return None;
        }

        let metadata = std::fs::metadata(&path).ok()?;
        let modified = metadata.modified().ok()?;
        let age = SystemTime::now().duration_since(modified).ok()?;

        if age > CACHE_TTL {
            std::fs::remove_file(&path).ok();
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Sauvegarder dans le cache
    pub fn set(
        &self,
        query: &str,
        category: &str,
        results: &[super::searcher::WebResult],
    ) -> Result<()> {
        let path = self.cache_key(query, category);
        let content = serde_json::to_string(results)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Nettoyer les entrées expirées
    pub fn cleanup(&self) {
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = SystemTime::now().duration_since(modified) {
                            if age > CACHE_TTL {
                                std::fs::remove_file(entry.path()).ok();
                            }
                        }
                    }
                }
            }
        }
    }
}
