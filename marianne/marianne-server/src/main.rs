// marianne-server/src/main.rs
// Point d'entrée du serveur HTTP Axum.

mod routes;
mod state;

use anyhow::Result;
use clap::Parser;
use marianne_core::state::AppState;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "marianne-server", about = "Marianne HTTP API Server")]
struct Cli {
    /// Adresse d'écoute (ex: 0.0.0.0:3000)
    #[arg(short, long, default_value = "127.0.0.1:3000")]
    bind: String,

    /// Répertoire de données (modèles, base vectorielle, historique)
    #[arg(short, long)]
    data_dir: Option<std::path::PathBuf>,

    /// Chemin vers le certificat TLS (PEM). Active HTTPS si fourni.
    #[arg(long)]
    tls_cert: Option<std::path::PathBuf>,

    /// Chemin vers la clé privée TLS (PEM). Requis si --tls-cert est fourni.
    #[arg(long)]
    tls_key: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("marianne_server=info".parse()?)
                .add_directive("marianne_core=info".parse()?),
        )
        .init();

    let data_dir = match cli.data_dir {
        Some(path) => path,
        None => dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!(
                "Impossible de déterminer le répertoire de données système.\n\
                 💡 Conseil : Spécifiez explicitement --data-dir=/chemin/vers/donnees"
            ))?
            .join("marianne"),
    };

    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(data_dir.join("models"))?;
    std::fs::create_dir_all(data_dir.join("db"))?;
    std::fs::create_dir_all(data_dir.join("graph"))?;
    std::fs::create_dir_all(data_dir.join("web_cache"))?;

    tracing::info!("Marianne Server — données dans : {:?}", data_dir);

    let core_state = AppState::new(data_dir);
    core_state.workspace.init().await?;

    // Initialiser la base de données historique (crée les tables si absentes)
    core_state.history.initialize().await?;

    // Installation et configuration automatique au démarrage
    if let Err(e) = marianne_core::setup::ensure_model_ready(&core_state).await {
        tracing::error!("❌ Échec de l'initialisation automatique : {:#}", e);
        tracing::warn!("Le serveur démarrera sans modèle chargé. Utilisez POST /api/v1/models/setup pour réessayer.");
    }

    let app_state = state::ServerState::new(core_state);

    let app = routes::build_router(app_state);

    // ─── Démarrage avec ou sans TLS ────────────────────────────────
    match (cli.tls_cert, cli.tls_key) {
        (Some(cert_path), Some(key_path)) => {
            // Mode HTTPS avec rustls
            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                &cert_path,
                &key_path,
            ).await?;

            let addr: std::net::SocketAddr = cli.bind.parse()?;
            tracing::info!("🔒 Écoute sur https://{} (TLS activé)", addr);
            axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        }
        (None, None) => {
            // Mode HTTP classique (inchangé)
            let listener = tokio::net::TcpListener::bind(&cli.bind).await?;
            tracing::info!("Écoute sur http://{}", cli.bind);
            axum::serve(listener, app).await?;
        }
        _ => {
            anyhow::bail!(
                "Les options --tls-cert et --tls-key doivent être fournies ensemble.\n\
                 💡 Exemple : marianne-server --tls-cert cert.pem --tls-key key.pem"
            );
        }
    }

    Ok(())
}

