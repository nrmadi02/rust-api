use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;
use tempfile::TempDir;
use uuid::Uuid;

use task_tools::application::activity_log::ActivityLogService;
use task_tools::application::auth::AuthService;
use task_tools::application::conversion::ConversionService;
use task_tools::application::jwt::JwtService;
use task_tools::application::login_attempt::LoginAttemptService;
use task_tools::application::password::hash_password;
use task_tools::build_router;
use task_tools::domain::storage::StorageRepository;
use task_tools::domain::user::UserRepository;
use task_tools::infrastructure::activity_log_repository::PgActivityLogRepository;
use task_tools::infrastructure::conversion_job_repository::PgConversionJobRepository;
use task_tools::infrastructure::local_storage_repository::LocalStorageRepository;
use task_tools::infrastructure::login_attempt_repository::PgLoginAttemptRepository;
use task_tools::infrastructure::pdf_validator::LopPdfValidator;
use task_tools::infrastructure::unoserver_client::UnoserverClient;
use task_tools::infrastructure::user_repository::PgUserRepository;
use task_tools::infrastructure::word_validator::SimpleWordValidator;
use task_tools::presentation::state::AppState;

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    pub storage_dir: TempDir,
    pub user_id: Uuid,
    pub token: String,
}

pub async fn setup_test_app() -> TestApp {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("failed to connect to test database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let storage_dir = tempfile::tempdir().expect("failed to create temp storage dir");
    let storage: Arc<dyn StorageRepository> =
        Arc::new(LocalStorageRepository::new(storage_dir.path(), 50));
    storage
        .ensure_layout()
        .await
        .expect("failed to init storage layout");

    let jwt_secret = "integration-test-jwt-secret-key-32chars!";
    let jwt_service = Arc::new(JwtService::new(jwt_secret.to_string(), 3600));
    let user_repo = Arc::new(PgUserRepository::new(pool.clone()));
    let login_attempt_service = Arc::new(LoginAttemptService::new(Arc::new(
        PgLoginAttemptRepository::new(pool.clone()),
    )));
    let auth_service = Arc::new(AuthService::new(
        user_repo.clone(),
        login_attempt_service,
        jwt_service.clone(),
    ));

    let email = format!("test-{}@example.com", Uuid::new_v4());
    let password_hash = hash_password("password123").expect("failed to hash password");
    let user = user_repo
        .create("Test User", &email, &password_hash)
        .await
        .expect("failed to create test user");

    let token = jwt_service
        .generate(user.id, &user.email)
        .expect("failed to generate jwt");

    let activity_log_repo = Arc::new(PgActivityLogRepository::new(pool.clone()));
    let conversion_service = Arc::new(ConversionService::new(
        Arc::new(PgConversionJobRepository::new(pool.clone())),
        activity_log_repo.clone(),
        storage,
        Arc::new(LopPdfValidator::new(50)),
        Arc::new(SimpleWordValidator::new(50)),
        Arc::new(UnoserverClient::new(
            std::env::var("UNOSERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            std::env::var("UNOSERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2003),
            60,
        )),
        storage_dir.path().to_path_buf(),
    ));
    let activity_log_service = Arc::new(ActivityLogService::new(activity_log_repo));

    let state = AppState {
        auth_service,
        jwt_service,
        conversion_service,
        activity_log_service,
    };

    TestApp {
        router: build_router().with_state(state),
        pool,
        storage_dir,
        user_id: user.id,
        token,
    }
}
