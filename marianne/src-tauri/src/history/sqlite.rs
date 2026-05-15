// src-tauri/src/history/sqlite.rs
use crate::prompts::system::ConversationTurn;
use anyhow::Result;
use std::path::Path;

/// Base de données SQLite pour l'historique des conversations
pub struct HistoryDb {
    db_path: std::path::PathBuf,
}

impl HistoryDb {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    /// Initialiser le schéma de la base de données
    pub async fn initialize(&self) -> Result<()> {
        let pool = self.connect().await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id TEXT NOT NULL,
                user_message TEXT NOT NULL,
                assistant_message TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_conv_id ON conversations(conversation_id)",
        )
        .execute(&pool)
        .await?;

        Ok(())
    }

    /// Sauvegarder un échange (question + réponse)
    pub async fn save_turn(
        &self,
        conversation_id: &str,
        user_message: &str,
        assistant_message: &str,
    ) -> Result<()> {
        let pool = self.connect().await?;
        sqlx::query(
            "INSERT INTO conversations (conversation_id, user_message, assistant_message) VALUES (?, ?, ?)",
        )
        .bind(conversation_id)
        .bind(user_message)
        .bind(assistant_message)
        .execute(&pool)
        .await?;

        Ok(())
    }

    /// Récupérer l'historique d'une conversation
    pub async fn get_conversation(&self, conversation_id: &str) -> Result<Vec<ConversationTurn>> {
        let pool = self.connect().await?;
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT user_message, assistant_message FROM conversations
             WHERE conversation_id = ? ORDER BY id ASC LIMIT 20",
        )
        .bind(conversation_id)
        .fetch_all(&pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(user, assistant)| ConversationTurn { user, assistant })
            .collect())
    }

    async fn connect(&self) -> Result<sqlx::SqlitePool> {
        let url = format!("sqlite:{}?mode=rwc", self.db_path.display());
        let pool = sqlx::SqlitePool::connect(&url).await?;
        Ok(pool)
    }
}
