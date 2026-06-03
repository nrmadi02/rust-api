use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::activity_log::{ActivityLog, ActivityLogRepository, ResourceType};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct PgActivityLogRepository {
    pool: PgPool,
}

impl PgActivityLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ActivityLogRepository for PgActivityLogRepository {
    async fn log_activity(&self, log: &ActivityLog) -> Result<(), DynError> {
        sqlx::query!(
            r#"
            INSERT INTO activity_logs (
                id,
                user_id,
                action,
                resource_type,
                resource_id,
                ip_address,
                user_agent,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            log.id,
            log.user_id,
            log.action,
            log.resource_type.as_ref().map(|rt| rt.as_str()),
            log.resource_id,
            log.ip_address,
            log.user_agent,
            log.metadata,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
        action: Option<&str>,
    ) -> Result<(Vec<ActivityLog>, u64), DynError> {
        let offset = (page.saturating_sub(1)) * per_page;

        let items = sqlx::query_as!(
            ActivityLog,
            r#"
            SELECT
                id,
                user_id,
                action,
                resource_type as "resource_type: ResourceType",
                resource_id,
                ip_address,
                user_agent,
                metadata,
                created_at
            FROM activity_logs
            WHERE user_id = $1
                AND ($2::text IS NULL OR action = $2)
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
            user_id,
            action,
            per_page as i64,
            offset as i64,
        )
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM activity_logs
            WHERE user_id = $1
                AND ($2::text IS NULL OR action = $2)
            "#,
            user_id,
            action,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((items, total as u64))
    }
}
