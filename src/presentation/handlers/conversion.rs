use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::presentation::dto::conversion::{
    ConversionJobResponse, ListJobsQuery, ListJobsResponse, UploadFileRequest,
};
use crate::presentation::middleware::auth::AuthUser;
use crate::presentation::response::api::{ApiResponse, PaginationMeta};
use crate::presentation::response::error::AppError;
use crate::presentation::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/convert/pdf-to-word",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    request_body(content = UploadFileRequest, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "PDF uploaded and draft conversion job created", body = inline(ApiResponse<ConversionJobResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn upload_pdf_to_word(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let (file_bytes, filename) = read_uploaded_file(&mut multipart).await?;

    let result = state
        .conversion_service
        .upload_pdf_to_word(auth.user_id, &file_bytes, &filename)
        .await?;

    let body = ApiResponse::success(
        true,
        "PDF uploaded successfully".to_string(),
        ConversionJobResponse::from(result.job),
    );

    Ok((StatusCode::ACCEPTED, Json(body)))
}

#[utoipa::path(
    get,
    path = "/api/v1/convert/jobs",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    params(ListJobsQuery),
    responses(
        (status = 200, description = "Conversion jobs retrieved successfully", body = inline(ApiResponse<ListJobsResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn list_jobs(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<ApiResponse<ListJobsResponse>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);

    let (jobs, total) = state
        .conversion_service
        .list_my_conversion_jobs(auth.user_id, page, per_page, query.status)
        .await?;

    let items = jobs.into_iter().map(ConversionJobResponse::from).collect();
    let response = ListJobsResponse {
        items,
        pagination: PaginationMeta::from_offset(page, per_page, total),
    };

    Ok(Json(ApiResponse::success(
        true,
        "Conversion jobs retrieved successfully".to_string(),
        response,
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/convert/jobs/{id}",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    params(
        ("id" = Uuid, Path, description = "Conversion job id"),
    ),
    responses(
        (status = 200, description = "Conversion job retrieved successfully", body = inline(ApiResponse<ConversionJobResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn get_job(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ConversionJobResponse>>, AppError> {
    let job = state
        .conversion_service
        .get_conversion_job_status(id, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success(
        true,
        "Conversion job retrieved successfully".to_string(),
        ConversionJobResponse::from(job),
    )))
}

#[utoipa::path(
    get,
    path = "/api/v1/convert/jobs/{id}/download",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    params(
        ("id" = Uuid, Path, description = "Conversion job id"),
    ),
    responses(
        (status = 200, description = "Converted file downloaded successfully"),
        (status = 400, description = "Job is not done yet"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn download_job(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response<Body>, AppError> {
    let file = state
        .conversion_service
        .download_converted_file(auth.user_id, id)
        .await?;

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, file.content_type)
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file.file_name),
        )
        .body(Body::from(file.bytes))
        .map_err(|_| AppError::InternalServerError)
}

#[utoipa::path(
    delete,
    path = "/api/v1/convert/jobs/{id}",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    params(
        ("id" = Uuid, Path, description = "Conversion job id"),
    ),
    responses(
        (status = 200, description = "Draft conversion job deleted successfully", body = inline(ApiResponse<serde_json::Value>)),
        (status = 400, description = "Job is not in draft status"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn delete_job(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    state
        .conversion_service
        .delete_draft_job(id, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success(
        true,
        "Draft conversion job deleted successfully".to_string(),
        serde_json::json!({ "id": id }),
    )))
}

async fn read_uploaded_file(multipart: &mut Multipart) -> Result<(Vec<u8>, String), AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| multipart_error(err.to_string()))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let filename = field.file_name().unwrap_or("upload.pdf").to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|err| multipart_error(err.to_string()))?;

        return Ok((bytes.to_vec(), filename));
    }

    Err(AppError::custom(
        StatusCode::BAD_REQUEST,
        "FILE_REQUIRED",
        "Multipart field `file` is required",
    ))
}

fn multipart_error(message: String) -> AppError {
    AppError::custom(StatusCode::BAD_REQUEST, "MULTIPART_ERROR", message)
}
