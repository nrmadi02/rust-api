use crate::presentation::dto::auth::{AuthResponse, RegisterRequest, UserResponse};
use crate::presentation::handlers::{auth, health, profile};
use crate::presentation::response::api::ApiResponse;
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Task Tools API",
        version = "0.1.0",
        description = "REST API — Scalar UI at /scalar"
    ),
    modifiers(&SecurityAddon),
    paths(
        health::health_check,
        auth::register,
        auth::login,
        profile::me,
    ),
    components(schemas(
        RegisterRequest,
        AuthResponse,
        UserResponse,
        ApiResponse<AuthResponse>,
        ApiResponse<UserResponse>,
    )),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Auth", description = "Auth endpoints"),
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.security_schemes.insert(
            "bearerAuth".to_string(),
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}
