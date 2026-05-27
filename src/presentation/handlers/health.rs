use axum::Json;

use crate::presentation::response::api::ApiResponse;

#[utoipa::path(
    get,
    path = "/",
    tag = "Health",
    responses(
        (status = 200, description = "OK", body = inline(crate::presentation::response::api::ApiResponse<String>))
    )
)]
pub async fn health_check() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::success(true, "OK".to_string(), "OK"))
}
