use std::sync::Arc;

use crate::application::auth::AuthService;
use crate::application::jwt::JwtService;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub jwt_service: Arc<JwtService>,
}
