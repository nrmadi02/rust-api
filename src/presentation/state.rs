use std::sync::Arc;

use crate::application::activity_log::ActivityLogService;
use crate::application::auth::AuthService;
use crate::application::conversion::ConversionService;
use crate::application::jwt::JwtService;

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub jwt_service: Arc<JwtService>,
    pub conversion_service: Arc<ConversionService>,
    pub activity_log_service: Arc<ActivityLogService>,
}
