use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::user::{User, UserRepository};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DynError> {
        let row = sqlx::query_as!(
            User,
            "SELECT id, name, email, password as password_hash, created_at, updated_at FROM users WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DynError> {
        let row = sqlx::query_as!(
            User,
            "SELECT id, name, email, password as password_hash, created_at, updated_at FROM users WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn create(&self, name: &str, email: &str, password_hash: &str) -> Result<User, DynError> {
        let row = sqlx::query_as!(
            User,
            "INSERT INTO users (name, email, password) VALUES ($1, $2, $3) RETURNING id, name, email, password as password_hash, created_at, updated_at",
            name,
            email,
            password_hash
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }
}
