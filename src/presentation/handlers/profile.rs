use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::presentation::dto::auth::UserResponse;
use crate::presentation::middleware::auth::AuthUser;
use crate::presentation::response::api::ApiResponse;
use crate::presentation::response::error::AppError;
use crate::presentation::state::AppState;

#[utoipa::path(
    get,
    path = "/api/profile/me",
    tag = "Auth",
    security(
        ("bearerAuth" = []),
    ),
    responses(
        (status = 200, description = "User retrieved successfully", body = inline(ApiResponse<UserResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn me(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<UserResponse>>, AppError> {
    let user = state
        .user_repository
        .get_user_by_id(auth.user_id)
        .await
        .map_err(|_| AppError::InternalServerError)?;

    if user.is_none() {
        return Err(AppError::Custom {
            status: StatusCode::NOT_FOUND,
            code: "USER_NOT_FOUND",
            message: "User not found".to_string(),
        });
    }

    let user = user.unwrap();

    Ok(Json(ApiResponse::success(
        true,
        "User retrieved successfully".to_string(),
        UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
        },
    )))
}
