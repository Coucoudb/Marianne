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

    let data_dir = cli
        .data_dir
        .unwrap_or_else(|| {
            dirs::data_dir()
                .expect("Impossible de trouver le répertoire de données")
                .join("marianne")
        });

    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(data_dir.join("models"))?;
    std::fs::create_dir_all(data_dir.join("db"))?;
    std::fs::create_dir_all(data_dir.join("graph"))?;
    std::fs::create_dir_all(data_dir.join("web_cache"))?;

    tracing::info!("Marianne Server — données dans : {:?}", data_dir);

    let core_state = AppState::new(data_dir);
    let app_state = state::ServerState::new(core_state);

    let app = routes::build_router(app_state);

    let listener = tokio::net::TcpListener::bind(&cli.bind).await?;
    tracing::info!("Écoute sur http://{}", cli.bind);
    axum::serve(listener, app).await?;

    Ok(())
}
