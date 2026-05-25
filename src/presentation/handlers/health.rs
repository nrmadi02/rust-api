use axum::Json;

use crate::presentation::response::api::ApiResponse;

pub async fn health_check() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::success(true, "OK".to_string(), "OK"))
}
