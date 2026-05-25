use crate::application::jwt::JwtService;
use crate::infrastructure::user_repository::UserRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub user_repository: Arc<UserRepository>,
    pub jwt_service: Arc<JwtService>,
}
