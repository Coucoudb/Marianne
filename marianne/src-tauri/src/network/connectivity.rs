// src-tauri/src/network/connectivity.rs

use std::time::{Duration, Instant};

/// Vérifier rapidement si internet est disponible
/// Requête HEAD légère vers un serveur gouvernemental
pub async fn is_online() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap_or_default();

    client
        .head("https://www.service-public.fr/")
        .send()
        .await
        .map(|r| r.status().is_success() || r.status().is_redirection())
        .unwrap_or(false)
}

/// État de connectivité mis en cache pour éviter des pings répétés
pub struct ConnectivityCache {
    last_check: parking_lot::Mutex<Option<(bool, Instant)>>,
    /// Ne re-pinger que toutes les 30 secondes
    check_interval: Duration,
}

impl ConnectivityCache {
    pub fn new() -> Self {
        Self {
            last_check: parking_lot::Mutex::new(None),
            check_interval: Duration::from_secs(30),
        }
    }

    pub async fn get_or_check(&self) -> bool {
        let cached = {
            let guard = self.last_check.lock();
            guard.as_ref().and_then(|(online, time)| {
                if time.elapsed() < self.check_interval {
                    Some(*online)
                } else {
                    None
                }
            })
        };

        if let Some(online) = cached {
            return online;
        }

        let online = is_online().await;
        *self.last_check.lock() = Some((online, Instant::now()));
        online
    }
}
