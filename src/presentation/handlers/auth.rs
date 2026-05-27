use axum::Json;
use axum::extract::State;

use crate::presentation::dto::auth::{AuthResponse, LoginRequest, RegisterRequest};
use crate::presentation::response::api::ApiResponse;
use crate::presentation::response::error::AppError;
use crate::presentation::state::AppState;
use crate::presentation::validation::validate_request;

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered", body = inline(ApiResponse<AuthResponse>)),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    validate_request(&body)?;

    let result = state
        .auth_service
        .register(&body.name, &body.email, &body.password)
        .await?;

    Ok(Json(ApiResponse::success(
        true,
        "User registered successfully".to_string(),
        result.into(),
    )))
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User logged in", body = inline(ApiResponse<AuthResponse>)),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, AppError> {
    validate_request(&body)?;

    let result = state
        .auth_service
        .login(&body.email, &body.password)
        .await?;

    Ok(Json(ApiResponse::success(
        true,
        "Login successful".into(),
        result.into(),
    )))
}
