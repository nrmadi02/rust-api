use axum::Json;
use axum::extract::State;

use crate::application::password::hash_password;
use crate::presentation::dto::auth::{AuthResponse, RegisterRequest, UserResponse};
use crate::presentation::response::api::ApiResponse;
use crate::presentation::response::error::AppError;
use crate::presentation::state::AppState;
use crate::presentation::validation::validate_request;

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
        return Err(AppError::BadRequest);
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
