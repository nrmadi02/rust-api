use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum ResourceType {
    ConversionJob,
    User,
    File,
    Webhook,
}

impl ResourceType {
    pub fn as_str(&self) -> &str {
        match self {
            ResourceType::ConversionJob => "conversion_job",
            ResourceType::User => "user",
            ResourceType::File => "file",
            ResourceType::Webhook => "webhook",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ActivityLog {
    pub id: Uuid,
    pub user_id: Uuid,
    pub action: String,
    pub resource_type: Option<ResourceType>,
    pub resource_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

impl ActivityLog {
    pub fn new(
        user_id: Uuid,
        action: impl Into<String>,
        resource_type: Option<ResourceType>,
        resource_id: Option<Uuid>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id,
            action: action.into(),
            resource_type,
            resource_id,
            ip_address: None,
            user_agent: None,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = Some(ua.into());
        self
    }

    pub fn with_metadata(mut self, metadata: JsonValue) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn upload_file(
        user_id: Uuid,
        job_id: Uuid,
        file_name: &str,
        file_size: i64,
    ) -> Self {
        let metadata = serde_json::json!({
            "file_name": file_name,
            "file_size_bytes": file_size,
        });

        Self::new(
            user_id,
            "upload_pdf",
            Some(ResourceType::ConversionJob),
            Some(job_id),
        )
        .with_metadata(metadata)
    }

    pub fn download_file(user_id: Uuid, job_id: Uuid, file_name: &str) -> Self {
        let metadata = serde_json::json!({
            "file_name": file_name,
        });

        Self::new(
            user_id,
            "download_file",
            Some(ResourceType::ConversionJob),
            Some(job_id),
        )
        .with_metadata(metadata)
    }

    pub fn delete_job(user_id: Uuid, job_id: Uuid) -> Self {
        Self::new(
            user_id,
            "delete_job",
            Some(ResourceType::ConversionJob),
            Some(job_id),
        )
    }

    pub fn login(user_id: Uuid, success: bool) -> Self {
        let action = if success { "login_success" } else { "login_failed" };
        let metadata = serde_json::json!({
            "success": success,
        });

        Self::new(user_id, action, Some(ResourceType::User), Some(user_id))
            .with_metadata(metadata)
    }

    pub fn logout(user_id: Uuid) -> Self {
        Self::new(user_id, "logout", Some(ResourceType::User), Some(user_id))
    }
}
