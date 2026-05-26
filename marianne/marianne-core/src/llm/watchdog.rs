use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub struct GenerationWatchdog {
    /// Nombre de tokens identiques consécutifs avant de déclarer une boucle
    max_repeat: usize,
    /// Temps maximum autorisé sans nouveau token (détecte le freeze CPU)
    token_timeout: Duration,
    /// Longueur minimale de réponse pour être considérée valide
    min_response_len: usize,

    // État interne
    last_token_time: Instant,
    recent_tokens: VecDeque<String>,
    consecutive_repeats: usize,
}

impl GenerationWatchdog {
    pub fn new() -> Self {
        Self {
            max_repeat: 8,
            token_timeout: Duration::from_secs(30),
            min_response_len: 20,
            last_token_time: Instant::now(),
            recent_tokens: VecDeque::with_capacity(16),
            consecutive_repeats: 0,
        }
    }

    /// Vérifier après chaque token — retourne Continue ou Abort
    pub fn check(&mut self, token: &str) -> WatchdogStatus {
        let now = Instant::now();

        // 1. Timeout entre tokens (CPU freeze ou deadlock)
        if now.duration_since(self.last_token_time) > self.token_timeout {
            tracing::warn!("Watchdog : timeout entre tokens détecté");
            return WatchdogStatus::Abort("Génération trop lente — timeout".to_string());
        }
        self.last_token_time = now;

        // 2. Détection de boucle de répétition
        if self.recent_tokens.back().map(|t| t == token).unwrap_or(false) {
            self.consecutive_repeats += 1;
            if self.consecutive_repeats >= self.max_repeat {
                tracing::warn!(
                    "Watchdog : boucle de répétition détectée ({} fois)",
                    self.consecutive_repeats
                );
                return WatchdogStatus::Abort("Boucle de répétition détectée".to_string());
            }
        } else {
            self.consecutive_repeats = 0;
        }

        // 3. Mettre à jour le buffer de tokens récents
        self.recent_tokens.push_back(token.to_string());
        if self.recent_tokens.len() > 16 {
            self.recent_tokens.pop_front();
        }

        WatchdogStatus::Continue
    }

    /// Valider la réponse finale avant de l'envoyer à l'utilisateur
    pub fn validate_response(&self, response: &str) -> ResponseValidity {
        if response.trim().len() < self.min_response_len {
            return ResponseValidity::TooShort;
        }
        // Détecter les réponses ne contenant que des tokens spéciaux
        let meaningful_chars = response
            .chars()
            .filter(|c| c.is_alphanumeric() || " .,;:!?".contains(*c))
            .count();
        if meaningful_chars < self.min_response_len {
            return ResponseValidity::Garbage;
        }
        ResponseValidity::Valid
    }
}

#[derive(Debug)]
pub enum WatchdogStatus {
    Continue,
    Abort(String),
}

#[derive(Debug)]
pub enum ResponseValidity {
    Valid,
    TooShort,
    Garbage,
}
