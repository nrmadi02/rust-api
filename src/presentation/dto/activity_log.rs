use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::activity_log::{ActivityLog, ResourceType};
use crate::presentation::response::api::PaginationMeta;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListActivityLogsQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub action: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ActivityLogResponse {
    pub id: uuid::Uuid,
    pub action: String,
    pub resource_type: Option<ResourceType>,
    pub resource_id: Option<uuid::Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ActivityLog> for ActivityLogResponse {
    fn from(log: ActivityLog) -> Self {
        Self {
            id: log.id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            ip_address: log.ip_address,
            user_agent: log.user_agent,
            metadata: log.metadata,
            created_at: log.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListActivityLogsResponse {
    pub items: Vec<ActivityLogResponse>,
    pub pagination: PaginationMeta,
}
