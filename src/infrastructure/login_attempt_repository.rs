use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::login_attempt::{LoginAttempt, LoginAttemptRepository};

const MAX_FAILED: i32 = 5;
const LOCK_DURATION_MINUTES: i32 = 15;

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

    async fn upsert_failure(&self, email: &str) -> Result<(), DynError> {
        sqlx::query!(
            r#"
            INSERT INTO login_attempts (email, failed_count, last_attempt)
            VALUES ($1, 1, NOW())
            ON CONFLICT (email) DO UPDATE
                SET failed_count = login_attempts.failed_count + 1,
                    last_attempt  = NOW(),
                    locked_until  = CASE
                        WHEN login_attempts.failed_count + 1 >= $2
                        THEN NOW() + ($3::int * INTERVAL '1 minute')
                        ELSE login_attempts.locked_until
                    END
            "#,
            email,
            MAX_FAILED,
            LOCK_DURATION_MINUTES,
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
