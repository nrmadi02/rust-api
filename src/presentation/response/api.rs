use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: T,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMeta {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedData<T> {
    pub items: Vec<T>,
    pub pagination: PaginationMeta,
}

impl PaginationMeta {
    pub fn from_offset(page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = if total == 0 {
            0
        } else {
            ((total as f64) / per_page as f64).ceil() as u32
        };
        Self {
            page,
            per_page,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn success(success: bool, message: String, data: T) -> Self {
        Self {
            data,
            success,
            message,
        }
    }

    pub fn paginated(
        message: impl Into<String>,
        items: Vec<T>,
        page: u32,
        per_page: u32,
        total: u64,
    ) -> ApiResponse<PaginatedData<T>> {
        ApiResponse {
            success: true,
            message: message.into(),
            data: PaginatedData {
                items,
                pagination: PaginationMeta::from_offset(page, per_page, total),
            },
        }
    }
}
