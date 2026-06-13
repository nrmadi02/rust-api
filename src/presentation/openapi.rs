use crate::domain::activity_log::ResourceType;
use crate::domain::conversion_job::{JobStatus, JobType};
use crate::presentation::dto::activity_log::{ActivityLogResponse, ListActivityLogsResponse};
use crate::presentation::dto::auth::{AuthResponse, RegisterRequest, UserResponse};
use crate::presentation::dto::conversion::{
    ConversionJobResponse, ListJobsResponse, UploadFileRequest, UploadImagesRequest,
};
use crate::presentation::handlers::{activity_log, auth, conversion, health, profile};
use crate::presentation::response::api::{ApiResponse, PaginationMeta};
use crate::presentation::response::error::ErrorResponse;
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
        activity_log::list_activity_logs,
        conversion::upload_pdf_to_word,
        conversion::upload_word_to_pdf,
        conversion::upload_image_to_pdf,
        conversion::upload_pdf_to_image,
        conversion::list_jobs,
        conversion::get_job,
        conversion::confirm_job,
        conversion::download_job,
        conversion::download_job_page,
        conversion::delete_job,
    ),
    components(schemas(
        RegisterRequest,
        AuthResponse,
        UserResponse,
        UploadFileRequest,
        UploadImagesRequest,
        ActivityLogResponse,
        ConversionJobResponse,
        ListActivityLogsResponse,
        ListJobsResponse,
        PaginationMeta,
        ResourceType,
        JobType,
        JobStatus,
        ApiResponse<AuthResponse>,
        ApiResponse<UserResponse>,
        ApiResponse<ConversionJobResponse>,
        ApiResponse<ListJobsResponse>,
        ApiResponse<ListActivityLogsResponse>,
        ErrorResponse,
    )),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Auth", description = "Auth endpoints"),
        (name = "Activity Logs", description = "User activity history endpoints"),
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
