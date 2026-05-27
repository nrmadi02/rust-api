use axum::Json;
use axum::extract::State;

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
    let user = state.auth_service.get_current_user(auth.user_id).await?;

    Ok(Json(ApiResponse::success(
        true,
        "User retrieved successfully".to_string(),
        user.into(),
    )))
}
