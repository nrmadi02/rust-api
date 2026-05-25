pub mod application;
pub mod config;
pub mod domain;
pub mod presentation;

use axum::Router;
use config::env::Config;

pub fn build_router() -> Router {
    presentation::router::create_router()
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    let app = build_router();
    let addr = format!("127.0.0.1:{}", config?.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    log::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
