use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DynError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DynError>;
    async fn create(&self, name: &str, email: &str, password_hash: &str) -> Result<User, DynError>;
}
