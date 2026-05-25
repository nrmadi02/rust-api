use crate::presentation::handlers::health;

use axum::{Router, routing::get};

pub fn create_router() -> Router {
    Router::new().route("/", get(health::health_check))
}
