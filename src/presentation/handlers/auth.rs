use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::application::password::{hash_password, verify_password};
use crate::presentation::dto::auth::{AuthResponse, LoginRequest, RegisterRequest, UserResponse};
use crate::presentation::response::api::ApiResponse;
use crate::presentation::response::error::AppError::{self};
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

    if state
        .user_repository
        .find_by_mail(&body.email)
        .await
        .map_err(|_| AppError::InternalServerError)?
        .is_some()
    {
        return Err(AppError::custom(
            StatusCode::CONFLICT,
            "EMAIL_ALREADY_REGISTERED",
            "Email is already registered",
        ));
    }

    let password_hash = hash_password(&body.password).map_err(|_| AppError::InternalServerError)?;

    let user = state
        .user_repository
        .create(&body.name, &body.email, &password_hash)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let token = state
        .jwt_service
        .generate(user.id, &user.email)
        .map_err(|_| AppError::InternalServerError)?;

    Ok(Json(ApiResponse::success(
        true,
        "User registered successfully".to_string(),
        AuthResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_service.expires_in(),
            user: UserResponse {
                id: user.id,
                email: user.email,
                name: user.name,
            },
        },
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

    if let Some(attempt) = state
        .login_attempt_service
        .check_locked(&body.email)
        .await
        .map_err(|_| AppError::InternalServerError)?
    {
        return Err(AppError::custom(
            StatusCode::TOO_MANY_REQUESTS,
            "TOO_MANY_ATTEMPTS",
            format!(
                "Account locked. Try again in {} seconds.",
                attempt.seconds_until_unlock()
            ),
        ));
    }

    let user = state
        .user_repository
        .find_by_mail(&body.email)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let Some(user) = user else {
        return Err(AppError::custom(
            StatusCode::UNAUTHORIZED,
            "INVALID_CREDENTIALS",
            "Invalid email or password",
        ));
    };

    let password_valid = verify_password(&body.password, &user.password_hash)
        .map_err(|_| AppError::InternalServerError)?;

    if !password_valid {
        state
            .login_attempt_service
            .record_failure(&body.email)
            .await
            .map_err(|_| AppError::InternalServerError)?;
        return Err(AppError::custom(
            StatusCode::UNAUTHORIZED,
            "INVALID_CREDENTIALS",
            "Invalid email or password",
        ));
    }

    state
        .login_attempt_service
        .reset(&body.email)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    let token = state
        .jwt_service
        .generate(user.id, &user.email)
        .map_err(|_| AppError::InternalServerError)?;

    Ok(Json(ApiResponse::success(
        true,
        "Login successful".into(),
        AuthResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt_service.expires_in(),
            user: UserResponse {
                id: user.id,
                email: user.email,
                name: user.name,
            },
        },
    )))
}
