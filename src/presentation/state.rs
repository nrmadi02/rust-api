use crate::application::jwt::JwtService;
use crate::application::login_attempt::LoginAttemptService;
use crate::infrastructure::user_repository::UserRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub user_repository: Arc<UserRepository>,
    pub jwt_service: Arc<JwtService>,
    pub login_attempt_service: Arc<LoginAttemptService>,
}
