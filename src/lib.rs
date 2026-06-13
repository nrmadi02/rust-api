pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use std::sync::Arc;

use axum::Router;
use config::env::Config;
use std::net::SocketAddr;

use self::application::activity_log::ActivityLogService;
use self::application::auth::AuthService;
use self::application::conversion::ConversionService;
use self::application::jwt::JwtService;
use self::application::login_attempt::LoginAttemptService;
use self::domain::storage::StorageRepository;
use self::infrastructure::activity_log_repository::PgActivityLogRepository;
use self::infrastructure::conversion_job_repository::PgConversionJobRepository;
use self::infrastructure::local_storage_repository::LocalStorageRepository;
use self::infrastructure::login_attempt_repository::PgLoginAttemptRepository;
use self::infrastructure::pdf_validator::LopPdfValidator;
use self::infrastructure::unoserver_client::UnoserverClient;
use self::infrastructure::user_repository::PgUserRepository;
use self::infrastructure::word_validator::SimpleWordValidator;
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
    let activity_log_repo = Arc::new(PgActivityLogRepository::new(pool.clone()));
    let conversion_service = Arc::new(ConversionService::new(
        Arc::new(PgConversionJobRepository::new(pool.clone())),
        activity_log_repo.clone(),
        storage,
        Arc::new(LopPdfValidator::new(config.max_upload_size_mb)),
        Arc::new(SimpleWordValidator::new(config.max_upload_size_mb)),
        Arc::new(UnoserverClient::new(
            config.uno_server_host,
            config.uno_server_port,
            config.uno_server_timeout_secs,
        )),
        config.storage_base_path.into(),
    ));
    let activity_log_service = Arc::new(ActivityLogService::new(activity_log_repo));

    let state = AppState {
        auth_service,
        jwt_service,
        conversion_service,
        activity_log_service,
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
