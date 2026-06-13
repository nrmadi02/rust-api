use axum::Json;
use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use uuid::Uuid;

use crate::presentation::dto::conversion::{
    ConversionJobResponse, ListJobsQuery, ListJobsResponse, UploadFileRequest,
    UploadImagesRequest,
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
    post,
    path = "/api/v1/convert/word-to-pdf",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    request_body(content = UploadFileRequest, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Word file uploaded and draft conversion job created", body = inline(ApiResponse<ConversionJobResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn upload_word_to_pdf(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let (file_bytes, filename) = read_uploaded_file(&mut multipart).await?;

    let result = state
        .conversion_service
        .upload_word_to_pdf(auth.user_id, &file_bytes, &filename)
        .await?;

    let body = ApiResponse::success(
        true,
        "Word file uploaded successfully".to_string(),
        ConversionJobResponse::from(result.job),
    );

    Ok((StatusCode::ACCEPTED, Json(body)))
}

#[utoipa::path(
    post,
    path = "/api/v1/convert/image-to-pdf",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    request_body(content = UploadImagesRequest, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Images uploaded and draft conversion job created", body = inline(ApiResponse<ConversionJobResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn upload_image_to_pdf(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let (files, order) = read_uploaded_images(&mut multipart).await?;

    let order_ref = order.as_deref();
    let result = state
        .conversion_service
        .upload_image_to_pdf(auth.user_id, &files, order_ref)
        .await?;

    let body = ApiResponse::success(
        true,
        format!("{} image(s) uploaded successfully", files.len()),
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
    query: Result<Query<ListJobsQuery>, QueryRejection>,
) -> Result<Json<ApiResponse<ListJobsResponse>>, AppError> {
    let Query(query) = query.map_err(|err| {
        AppError::custom(StatusCode::BAD_REQUEST, "INVALID_QUERY", err.to_string())
    })?;
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
    post,
    path = "/api/v1/convert/jobs/{id}/confirm",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    params(
        ("id" = Uuid, Path, description = "Conversion job id"),
    ),
    responses(
        (status = 202, description = "Draft job confirmed and conversion queued", body = inline(ApiResponse<ConversionJobResponse>)),
        (status = 400, description = "Job is not in draft status"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn confirm_job(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let job = state
        .conversion_service
        .enqueue_conversion_job(id, auth.user_id)
        .await?;

    let body = ApiResponse::success(
        true,
        "Conversion job confirmed and queued".to_string(),
        ConversionJobResponse::from(job),
    );

    Ok((StatusCode::ACCEPTED, Json(body)))
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

#[utoipa::path(
    post,
    path = "/api/v1/convert/pdf-to-image",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    request_body(content = UploadFileRequest, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "PDF uploaded and draft pdf-to-image job created", body = inline(ApiResponse<ConversionJobResponse>)),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn upload_pdf_to_image(
    auth: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let (file_bytes, filename) = read_uploaded_file(&mut multipart).await?;

    let result = state
        .conversion_service
        .upload_pdf_to_image(auth.user_id, &file_bytes, &filename)
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
    path = "/api/v1/convert/jobs/{id}/pages/{page}",
    tag = "Conversion",
    security(
        ("bearerAuth" = []),
    ),
    params(
        ("id" = Uuid, Path, description = "Conversion job id"),
        ("page" = u32, Path, description = "Page number (1-indexed)"),
    ),
    responses(
        (status = 200, description = "Page image downloaded successfully"),
        (status = 400, description = "Invalid page number"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Job or page not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn download_job_page(
    auth: AuthUser,
    State(state): State<AppState>,
    Path((id, page)): Path<(Uuid, u32)>,
) -> Result<Response<Body>, AppError> {
    if page == 0 {
        return Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            "INVALID_PAGE",
            "Page number must be >= 1".to_string(),
        ));
    }

    let file = state
        .conversion_service
        .download_page(auth.user_id, id, page)
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

async fn read_uploaded_images(
    multipart: &mut Multipart,
) -> Result<(Vec<(Vec<u8>, String)>, Option<Vec<usize>>), AppError> {
    let mut files: Vec<(Vec<u8>, String)> = Vec::new();
    let mut order: Option<Vec<usize>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| multipart_error(err.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "file" {
            let filename = field.file_name().unwrap_or("image.jpg").to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|err| multipart_error(err.to_string()))?;
            files.push((bytes.to_vec(), filename));
        } else if name == "order" {
            let text = field
                .text()
                .await
                .map_err(|err| multipart_error(err.to_string()))?;
            let parsed: Result<Vec<usize>, _> = text
                .split(',')
                .map(|s| s.trim().parse::<usize>())
                .collect();
            order = Some(parsed.map_err(|_| {
                AppError::custom(
                    StatusCode::BAD_REQUEST,
                    "INVALID_ORDER",
                    "Order must be comma-separated indices (e.g., \"0,2,1\")",
                )
            })?);
        }
    }

    if files.is_empty() {
        return Err(AppError::custom(
            StatusCode::BAD_REQUEST,
            "FILE_REQUIRED",
            "At least one multipart field `file` is required",
        ));
    }

    Ok((files, order))
}
