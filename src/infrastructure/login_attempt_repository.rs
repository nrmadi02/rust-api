use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::login_attempt::{LoginAttempt, LoginAttemptRepository};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct PgLoginAttemptRepository {
    pool: PgPool,
}

impl PgLoginAttemptRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoginAttemptRepository for PgLoginAttemptRepository {
    async fn find(&self, email: &str) -> Result<Option<LoginAttempt>, DynError> {
        let row = sqlx::query_as!(
            LoginAttempt,
            "SELECT email, failed_count, locked_until FROM login_attempts WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn save(&self, attempt: &LoginAttempt) -> Result<(), DynError> {
        sqlx::query!(
            r#"
            INSERT INTO login_attempts (email, failed_count, locked_until, last_attempt)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (email) DO UPDATE
                SET failed_count = EXCLUDED.failed_count,
                    locked_until = EXCLUDED.locked_until,
                    last_attempt = NOW()
            "#,
            attempt.email,
            attempt.failed_count,
            attempt.locked_until,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, email: &str) -> Result<(), DynError> {
        sqlx::query!("DELETE FROM login_attempts WHERE email = $1", email)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
