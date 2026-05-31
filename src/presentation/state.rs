use std::sync::Arc;

use crate::application::auth::AuthService;
use crate::application::jwt::JwtService;
use crate::domain::storage::StorageRepository;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub jwt_service: Arc<JwtService>,
    pub storage: Arc<dyn StorageRepository>,
}
