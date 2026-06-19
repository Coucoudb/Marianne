// marianne-core/src/history/sqlite.rs
use crate::prompts::system::ConversationTurn;
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Résumé d'une conversation pour la liste latérale
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub last_message_at: i64,
    pub message_count: i64,
    pub first_message_preview: String,
}

/// Message individuel pour le client (format plat)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

/// Fait mémorisé entre les sessions
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryNote {
    pub key: String,
    pub value: String,
    pub source_conversation_id: Option<String>,
    pub updated_at: i64,
}

/// Base de données SQLite pour l'historique des conversations
pub struct HistoryDb {
    db_path: std::path::PathBuf,
    initialized: AtomicBool,
    key: [u8; 32],
}

impl HistoryDb {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
            initialized: AtomicBool::new(false),
            key: crate::crypto::get_db_key(),
        }
    }

    /// Initialiser le schéma de la base de données
    pub async fn initialize(&self) -> Result<()> {
        let pool = self.raw_connect().await?;

        // Table principale des conversations (enrichie)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                user_id TEXT NOT NULL DEFAULT 'default',
                user_message TEXT NOT NULL,
                assistant_message TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        // Migration : ajouter user_id si la table existait avant
        Self::add_column_if_missing(&pool, "conversations", "user_id", "TEXT NOT NULL DEFAULT 'default'").await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_conv_id ON conversations(conversation_id)",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_conv_user ON conversations(user_id)",
        )
        .execute(&pool)
        .await?;

        // Table des titres de conversations
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversation_meta (
                conversation_id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL DEFAULT 'default',
                title TEXT NOT NULL DEFAULT '',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        // Migration : ajouter user_id si la table existait avant
        Self::add_column_if_missing(&pool, "conversation_meta", "user_id", "TEXT NOT NULL DEFAULT 'default'").await?;

        // Table des mémoires persistantes (inter-sessions)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL UNIQUE,
                value TEXT NOT NULL,
                source_conversation_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        self.initialized.store(true, Ordering::SeqCst);
        tracing::info!("✅ Base de données historique initialisée");
        Ok(())
    }

    /// Ajouter une colonne à une table si elle n'existe pas encore.
    /// SQLite ne supporte pas `ALTER TABLE ... ADD COLUMN IF NOT EXISTS`,
    /// on ignore donc l'erreur si la colonne est déjà présente.
    ///
    /// # Security
    /// SQLite does not support bound parameters for DDL identifiers, so `table`,
    /// `column`, and `typedef` are interpolated directly into the SQL string.
    /// This function is **private** and called exclusively with hard-coded string
    /// literals — never with user-supplied input. Do not expose it publicly.
    async fn add_column_if_missing(pool: &sqlx::SqlitePool, table: &str, column: &str, typedef: &str) -> Result<()> {
        let _ = sqlx::query(&format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, typedef))
            .execute(pool)
            .await; // ignore l'erreur "duplicate column name"
        Ok(())
    }

    /// Connecter et auto-initialiser si nécessaire
    async fn connect(&self) -> Result<sqlx::SqlitePool> {
        let pool = self.raw_connect().await?;
        if !self.initialized.load(Ordering::SeqCst) {
            self.initialize().await?;
        }
        Ok(pool)
    }

    /// Connexion brute sans auto-init (pour éviter la récursion)
    async fn raw_connect(&self) -> Result<sqlx::SqlitePool> {
        let url = format!("sqlite:{}?mode=rwc", self.db_path.display());
        let pool = sqlx::SqlitePool::connect(&url).await?;
        Ok(pool)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Conversations — écriture
    // ═══════════════════════════════════════════════════════════════════════

    /// Sauvegarder un échange (question + réponse)
    pub async fn save_turn(
        &self,
        conversation_id: &str,
        user_id: &str,
        user_message: &str,
        assistant_message: &str,
    ) -> Result<()> {
        let pool = self.connect().await?;

        let enc_user = crate::crypto::encrypt(&self.key, user_message)?;
        let enc_assistant = crate::crypto::encrypt(&self.key, assistant_message)?;

        sqlx::query(
            "INSERT INTO conversations (conversation_id, user_id, user_message, assistant_message) VALUES (?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(&enc_user)
        .bind(&enc_assistant)
        .execute(&pool)
        .await?;

        // Mettre à jour / créer la métadonnée de la conversation
        let title = generate_title(user_message);
        sqlx::query(
            "INSERT INTO conversation_meta (conversation_id, user_id, title, created_at, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT(conversation_id) DO UPDATE SET updated_at = CURRENT_TIMESTAMP",
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(&title)
        .execute(&pool)
        .await?;

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Conversations — lecture
    // ═══════════════════════════════════════════════════════════════════════

    /// Récupérer l'historique d'une conversation (format interne pour le prompt)
    pub async fn get_conversation(&self, conversation_id: &str, user_id: &str) -> Result<Vec<ConversationTurn>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT user_message, assistant_message FROM conversations
             WHERE conversation_id = ? AND user_id = ? ORDER BY id ASC LIMIT 50",
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_all(&pool)
        .await?;

        rows.into_iter()
            .map(|(user, assistant)| {
                let user = maybe_decrypt(&self.key, user)?;
                let assistant = maybe_decrypt(&self.key, assistant)?;
                Ok(ConversationTurn { user, assistant })
            })
            .collect()
    }

    /// Récupérer l'historique d'une conversation au format client
    /// Retourne des messages plats {role, content, timestamp}
    pub async fn get_conversation_messages(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<Vec<ConversationMessage>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT user_message, assistant_message, created_at FROM conversations
             WHERE conversation_id = ? AND user_id = ? ORDER BY id ASC LIMIT 100",
        )
        .bind(conversation_id)
        .bind(user_id)
        .fetch_all(&pool)
        .await?;

        let mut messages = Vec::new();
        for (user, assistant, created_at) in rows {
            let ts = parse_sqlite_datetime(&created_at);
            let user = maybe_decrypt(&self.key, user)?;
            let assistant = maybe_decrypt(&self.key, assistant)?;
            messages.push(ConversationMessage {
                role: "user".to_string(),
                content: user,
                timestamp: ts,
            });
            messages.push(ConversationMessage {
                role: "assistant".to_string(),
                content: assistant,
                timestamp: ts,
            });
        }
        Ok(messages)
    }

    /// Lister toutes les conversations avec résumé
    pub async fn list_conversations(&self, user_id: &str) -> Result<Vec<ConversationSummary>> {
        let pool = self.connect().await?;

        let rows = sqlx::query_as::<_, (String, String, String, i32)>(
            "SELECT
                c.conversation_id,
                COALESCE(m.title, SUBSTR(MIN(c.user_message), 1, 80)) as title,
                COALESCE(m.updated_at, MAX(c.created_at)) as last_at,
                COUNT(*) as msg_count
             FROM conversations c
             LEFT JOIN conversation_meta m ON c.conversation_id = m.conversation_id
             WHERE c.user_id = ?
             GROUP BY c.conversation_id
             ORDER BY last_at DESC
             LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(&pool)
        .await?;

        let mut summaries = Vec::new();
        for (conv_id, title, last_at, msg_count) in rows {
            // Récupérer le premier message pour le preview
            let first_msg_raw = sqlx::query_as::<_, (String,)>(
                "SELECT user_message FROM conversations
                 WHERE conversation_id = ? AND user_id = ? ORDER BY id ASC LIMIT 1",
            )
            .bind(&conv_id)
            .bind(user_id)
            .fetch_optional(&pool)
            .await?
            .map(|(m,)| m)
            .unwrap_or_default();
            let first_msg = maybe_decrypt(&self.key, first_msg_raw).unwrap_or_default();

            let ts = parse_sqlite_datetime(&last_at);
            summaries.push(ConversationSummary {
                id: conv_id,
                title,
                created_at: ts,
                last_message_at: ts,
                message_count: msg_count as i64,
                first_message_preview: first_msg.chars().take(100).collect(),
            });
        }

        Ok(summaries)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Mémoire persistante (inter-sessions)
    // ═══════════════════════════════════════════════════════════════════════

    /// Sauvegarder ou mettre à jour un fait mémorisé
    pub async fn save_memory(
        &self,
        key: &str,
        value: &str,
        conversation_id: Option<&str>,
    ) -> Result<()> {
        let pool = self.connect().await?;
        sqlx::query(
            "INSERT INTO memories (key, value, source_conversation_id, updated_at)
             VALUES (?, ?, ?, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = CURRENT_TIMESTAMP",
        )
        .bind(key)
        .bind(value)
        .bind(conversation_id)
        .execute(&pool)
        .await?;
        Ok(())
    }

    /// Récupérer toutes les mémoires persistantes
    pub async fn get_memories(&self) -> Result<Vec<MemoryNote>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String, Option<String>, String)>(
            "SELECT key, value, source_conversation_id, updated_at
             FROM memories ORDER BY updated_at DESC LIMIT 30",
        )
        .fetch_all(&pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(key, value, source, updated)| MemoryNote {
                key,
                value,
                source_conversation_id: source,
                updated_at: parse_sqlite_datetime(&updated),
            })
            .collect())
    }

    /// Récupérer les N derniers résumés de conversations pour mémoire cross-session
    pub async fn get_recent_summaries(&self, limit: usize) -> Result<Vec<String>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT conversation_id, user_message FROM conversations
             GROUP BY conversation_id
             ORDER BY MAX(id) DESC
             LIMIT ?",
        )
        .bind(limit as i32)
        .fetch_all(&pool)
        .await?;

        // Construire un résumé léger de chaque conversation récente
        let mut summaries = Vec::new();
        for (conv_id, first_msg_raw) in rows {
            let first_msg = maybe_decrypt(&self.key, first_msg_raw).unwrap_or_default();
            let preview: String = first_msg.chars().take(60).collect();
            let count = sqlx::query_as::<_, (i32,)>(
                "SELECT COUNT(*) FROM conversations WHERE conversation_id = ?",
            )
            .bind(&conv_id)
            .fetch_one(&pool)
            .await
            .map(|(c,)| c)
            .unwrap_or(0);

            summaries.push(format!("- {} ({} échanges)", preview, count));
        }

        Ok(summaries)
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Fonctions utilitaires
// ═══════════════════════════════════════════════════════════════════════════

/// Decrypt `s` if it looks encrypted; return it as-is for backward-compat plaintext rows.
fn maybe_decrypt(key: &[u8; 32], s: String) -> Result<String> {
    if crate::crypto::is_encrypted(&s) {
        crate::crypto::decrypt(key, &s)
    } else {
        Ok(s)
    }
}

/// Générer un titre court à partir du premier message
fn generate_title(message: &str) -> String {
    let clean: String = message
        .chars()
        .take(60)
        .collect();

    // Couper au dernier espace
    if let Some(pos) = clean.rfind(' ') {
        if pos > 20 {
            return format!("{}…", &clean[..pos]);
        }
    }

    if clean.len() < message.len() {
        format!("{}…", clean)
    } else {
        clean
    }
}

/// Parser un datetime SQLite en timestamp Unix
fn parse_sqlite_datetime(dt: &str) -> i64 {
    // SQLite retourne "YYYY-MM-DD HH:MM:SS" ou un timestamp ISO
    chrono::NaiveDateTime::parse_from_str(dt, "%Y-%m-%d %H:%M:%S")
        .map(|ndt| ndt.and_utc().timestamp())
        .unwrap_or_else(|_| {
            // Essayer le format ISO
            chrono::DateTime::parse_from_rfc3339(dt)
                .map(|d| d.timestamp())
                .unwrap_or_else(|_| {
                    // Dernier recours : essayer de parser comme un nombre
                    dt.parse::<i64>().unwrap_or(0)
                })
        })
}
