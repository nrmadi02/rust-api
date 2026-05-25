pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use std::sync::Arc;

use axum::Router;
use config::env::Config;

use self::application::jwt::JwtService;
use self::infrastructure::user_repository::UserRepository;
use self::presentation::state::AppState;

pub fn build_router() -> Router<AppState> {
    presentation::router::create_router()
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let pool = sqlx::PgPool::connect(&config.database_url).await?;
    let state = AppState {
        user_repository: Arc::new(UserRepository::new(pool)),
        jwt_service: Arc::new(JwtService::new(
            config.jwt_secret.clone(),
            config.jwt_expires_in as i64,
        )),
    };
    let app = build_router().with_state(state);
    let addr = format!("127.0.0.1:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    log::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
