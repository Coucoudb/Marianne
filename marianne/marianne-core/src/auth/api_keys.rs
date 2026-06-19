// marianne-core/src/auth/api_keys.rs
// Stockage et lookup des clés API multi-utilisateurs.
//
// Security: les clés brutes ne sont jamais stockées.
// Seul SHA-256(raw_key) en hex est persisté (OWASP A02).

use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Admin,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Admin => "admin",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => Role::Admin,
            _ => Role::User,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApiKeyRecord {
    pub user_id: String,
    pub label: String,
    pub role: Role,
    pub created_at: String,
    pub revoked: bool,
}

pub struct ApiKeyDb {
    db_path: std::path::PathBuf,
}

impl ApiKeyDb {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    async fn connect(&self) -> Result<sqlx::SqlitePool> {
        let url = format!("sqlite:{}?mode=rwc", self.db_path.display());
        Ok(sqlx::SqlitePool::connect(&url).await?)
    }

    pub async fn initialize(&self) -> Result<()> {
        let pool = self.connect().await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS api_keys (
                key_hash   TEXT PRIMARY KEY,
                user_id    TEXT NOT NULL,
                label      TEXT NOT NULL DEFAULT '',
                role       TEXT NOT NULL DEFAULT 'user',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                revoked    INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_keys_user ON api_keys(user_id)",
        )
        .execute(&pool)
        .await?;
        Ok(())
    }

    /// Hash a raw key with SHA-256, return lower-hex string.
    pub fn hash_key(raw_key: &str) -> String {
        format!("{:x}", Sha256::digest(raw_key.as_bytes()))
    }

    /// Look up an API key. Returns None if not found or revoked.
    pub async fn lookup(&self, raw_key: &str) -> Result<Option<ApiKeyRecord>> {
        let key_hash = Self::hash_key(raw_key);
        let pool = self.connect().await?;
        let row = sqlx::query_as::<_, (String, String, String, i64)>(
            "SELECT user_id, label, role, revoked FROM api_keys WHERE key_hash = ?",
        )
        .bind(&key_hash)
        .fetch_optional(&pool)
        .await?;

        Ok(row.and_then(|(user_id, label, role, revoked)| {
            if revoked != 0 {
                return None;
            }
            Some(ApiKeyRecord {
                user_id,
                label,
                role: Role::from_str(&role),
                created_at: String::new(),
                revoked: false,
            })
        }))
    }

    /// Insert a new API key. `raw_key` is provided by the caller (already generated).
    pub async fn insert(
        &self,
        raw_key: &str,
        user_id: &str,
        label: &str,
        role: Role,
    ) -> Result<()> {
        let key_hash = Self::hash_key(raw_key);
        let pool = self.connect().await?;
        sqlx::query(
            "INSERT INTO api_keys (key_hash, user_id, label, role) VALUES (?, ?, ?, ?)",
        )
        .bind(&key_hash)
        .bind(user_id)
        .bind(label)
        .bind(role.as_str())
        .execute(&pool)
        .await?;
        Ok(())
    }

    /// List all keys for a user (without the hash).
    pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
            "SELECT user_id, label, role, created_at, revoked FROM api_keys \
             WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(user_id, label, role, created_at, revoked)| ApiKeyRecord {
                user_id,
                label,
                role: Role::from_str(&role),
                created_at,
                revoked: revoked != 0,
            })
            .collect())
    }

    /// List ALL keys (admin only — filtering done at route level).
    pub async fn list_all(&self) -> Result<Vec<ApiKeyRecord>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String, String, String, i64)>(
            "SELECT user_id, label, role, created_at, revoked \
             FROM api_keys ORDER BY created_at DESC",
        )
        .fetch_all(&pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(user_id, label, role, created_at, revoked)| ApiKeyRecord {
                user_id,
                label,
                role: Role::from_str(&role),
                created_at,
                revoked: revoked != 0,
            })
            .collect())
    }

    /// Revoke a key by its hash (admin operation).
    pub async fn revoke(&self, key_hash: &str) -> Result<bool> {
        let pool = self.connect().await?;
        let result = sqlx::query("UPDATE api_keys SET revoked = 1 WHERE key_hash = ?")
            .bind(key_hash)
            .execute(&pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
