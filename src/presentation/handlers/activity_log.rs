use axum::Json;
use axum::extract::{Query, State};

use crate::presentation::dto::activity_log::{
    ActivityLogResponse, ListActivityLogsQuery, ListActivityLogsResponse,
};
use crate::presentation::middleware::auth::AuthUser;
use crate::presentation::response::api::{ApiResponse, PaginationMeta};
use crate::presentation::response::error::AppError;
use crate::presentation::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/me/activity-logs",
    tag = "Activity Logs",
    security(
        ("bearerAuth" = []),
    ),
    params(ListActivityLogsQuery),
    responses(
        (status = 200, description = "Activity logs retrieved successfully", body = inline(ApiResponse<ListActivityLogsResponse>)),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn list_activity_logs(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListActivityLogsQuery>,
) -> Result<Json<ApiResponse<ListActivityLogsResponse>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);

    let (logs, total) = state
        .activity_log_service
        .list_my_activity_logs(
            auth.user_id,
            page,
            per_page,
            query.action.as_deref(),
        )
        .await?;

    let items = logs.into_iter().map(ActivityLogResponse::from).collect();
    let response = ListActivityLogsResponse {
        items,
        pagination: PaginationMeta::from_offset(page, per_page, total),
    };

    Ok(Json(ApiResponse::success(
        true,
        "Activity logs retrieved successfully".to_string(),
        response,
    )))
}
