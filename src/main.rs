use std::sync::Arc;

use clap::Parser;
use crabbit::config::Config;
use crabbit::state::EngineState;
use crabbit::{api, crawler};

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = Config::load(&cli.config)?;
    let addr = format!("{}:{}", config.server.host, config.server.port);

    let state = Arc::new(EngineState::new(config)?);

    tokio::spawn(crawler::worker::run(state.clone()));

    let router = api::routes::build_router(state);

    tracing::info!(%addr, "starting crabbit");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
