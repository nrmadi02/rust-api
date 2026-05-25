use crate::presentation::handlers::health;

use axum::{Router, routing::get};

use super::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new().route("/", get(health::health_check))
}
