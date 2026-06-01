pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use std::sync::Arc;

use axum::Router;
use config::env::Config;
use std::net::SocketAddr;

use self::application::auth::AuthService;
use self::application::jwt::JwtService;
use self::application::login_attempt::LoginAttemptService;
use self::domain::storage::StorageRepository;
use self::infrastructure::local_storage_repository::LocalStorageRepository;
use self::infrastructure::login_attempt_repository::PgLoginAttemptRepository;
use self::infrastructure::user_repository::PgUserRepository;
use self::presentation::state::AppState;

pub fn build_router() -> Router<AppState> {
    presentation::router::create_router()
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let pool = sqlx::PgPool::connect(&config.database_url).await?;

    let storage: Arc<dyn StorageRepository> = Arc::new(LocalStorageRepository::new(
        &config.storage_base_path,
        config.max_upload_size_mb,
    ));

    storage
        .ensure_layout()
        .await
        .map_err(|e| format!("Storage init error: {:?}", e))?;

    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));
    let jwt_service = Arc::new(JwtService::new(
        config.jwt_secret.clone(),
        config.jwt_expires_in as i64,
    ));
    let login_attempt_service = Arc::new(LoginAttemptService::new(Arc::new(
        PgLoginAttemptRepository::new(pool.clone()),
    )));
    let auth_service = Arc::new(AuthService::new(
        user_repo,
        login_attempt_service,
        jwt_service.clone(),
    ));

    let state = AppState {
        auth_service,
        jwt_service,
    };

    let app = build_router().with_state(state);
    let addr = format!("127.0.0.1:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    log::info!("Listening on {}", addr);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
