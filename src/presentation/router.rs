use crate::presentation::handlers::health;
use crate::presentation::openapi::ApiDoc;
use tower_governor::{
    GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor,
};

use axum::routing::post;
use axum::{Router, routing::get};
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use super::handlers::{auth, profile};
use super::state::AppState;

pub fn create_router() -> Router<AppState> {
    let login_governer_config = GovernorConfigBuilder::default()
        .per_second(2)
        .burst_size(5)
        .use_headers()
        .key_extractor(SmartIpKeyExtractor)
        .finish()
        .unwrap();

    let auth_routes = Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .layer(GovernorLayer::new(login_governer_config));

    let public = Router::new()
        .route("/", get(health::health_check))
        .merge(auth_routes);

    let protected = Router::new().route("/api/profile/me", get(profile::me));

    let doc = Scalar::with_url("/scalar", ApiDoc::openapi());

    Router::new().merge(public).merge(protected).merge(doc)
}
