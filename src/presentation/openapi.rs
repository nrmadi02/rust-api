use crate::domain::conversion_job::{JobStatus, JobType};
use crate::presentation::dto::auth::{AuthResponse, RegisterRequest, UserResponse};
use crate::presentation::dto::conversion::{
    ConversionJobResponse, ListJobsResponse, UploadFileRequest,
};
use crate::presentation::handlers::{auth, conversion, health, profile};
use crate::presentation::response::api::{ApiResponse, PaginationMeta};
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
        conversion::upload_pdf_to_word,
        conversion::list_jobs,
        conversion::get_job,
        conversion::download_job,
        conversion::delete_job,
    ),
    components(schemas(
        RegisterRequest,
        AuthResponse,
        UserResponse,
        UploadFileRequest,
        ConversionJobResponse,
        ListJobsResponse,
        PaginationMeta,
        JobType,
        JobStatus,
        ApiResponse<AuthResponse>,
        ApiResponse<UserResponse>,
        ApiResponse<ConversionJobResponse>,
        ApiResponse<ListJobsResponse>,
    )),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Auth", description = "Auth endpoints"),
        (name = "Conversion", description = "File conversion endpoints"),
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
