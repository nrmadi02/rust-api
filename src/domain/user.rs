use chrono::NaiveDateTime;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "user_status", rename_all = "lowercase")]
pub enum UserStatus {
    Pending,
    Approved,
    Rejected,
    Suspended,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub status: UserStatus,
    pub approved_by: Option<Uuid>,
    pub rejected_at: Option<NaiveDateTime>,
    pub rejected_by: Option<Uuid>,
    pub rejection_reason: Option<String>,
    pub role: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_status(&self, user_id: Uuid) -> Result<UserStatus, DynError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DynError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DynError>;
    async fn create(&self, name: &str, email: &str, password_hash: &str) -> Result<User, DynError>;
}
