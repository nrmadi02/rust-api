use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::conversion_job::{ConversionJob, ConversionJobRepository, JobStatus, JobType};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct PgConversionJobRepository {
    pool: PgPool,
}

impl PgConversionJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConversionJobRepository for PgConversionJobRepository {
    async fn create_job(&self, job: &ConversionJob) -> Result<ConversionJob, DynError> {
        let row = sqlx::query_as!(
            ConversionJob,
            r#"
            INSERT INTO conversion_jobs (id, user_id, job_type, status, input_file, output_file, error_message)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id,
                user_id,
                job_type as "job_type: JobType",
                status as "status: JobStatus",
                input_file,
                output_file,
                error_message,
                duration_ms,
                created_at,
                updated_at
            "#,
            job.id,
            job.user_id,
            job.job_type as JobType,
            job.status as JobStatus,
            job.input_file,
            job.output_file,
            job.error_message,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ConversionJob>, DynError> {
        let row = sqlx::query_as!(
            ConversionJob,
            r#"
            SELECT
                id,
                user_id,
                job_type as "job_type: JobType",
                status as "status: JobStatus",
                input_file,
                output_file,
                error_message,
                duration_ms,
                created_at,
                updated_at
            FROM conversion_jobs
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
    ) -> Result<(Vec<ConversionJob>, u64), DynError> {
        let offset = (page.saturating_sub(1)) * per_page;

        let items = sqlx::query_as!(
            ConversionJob,
            r#"
            SELECT
                id,
                user_id,
                job_type as "job_type: JobType",
                status as "status: JobStatus",
                input_file,
                output_file,
                error_message,
                duration_ms,
                created_at,
                updated_at
            FROM conversion_jobs
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            per_page as i64,
            offset as i64,
        )
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM conversion_jobs WHERE user_id = $1"#,
            user_id,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((items, total as u64))
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: JobStatus,
        output_file: Option<&str>,
        error_message: Option<&str>,
        duration_ms: Option<i32>,
    ) -> Result<ConversionJob, DynError> {
        let row = sqlx::query_as!(
            ConversionJob,
            r#"
            UPDATE conversion_jobs
            SET status = $2,
                output_file = $3,
                error_message = $4,
                duration_ms = COALESCE($5, duration_ms),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                user_id,
                job_type as "job_type: JobType",
                status as "status: JobStatus",
                input_file,
                output_file,
                error_message,
                duration_ms,
                created_at,
                updated_at
            "#,
            id,
            status as JobStatus,
            output_file,
            error_message,
            duration_ms,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or("Job not found")?;

        Ok(row)
    }

    async fn delete_draft(&self, id: Uuid) -> Result<(), DynError> {
        let result = sqlx::query!(
            r#"DELETE FROM conversion_jobs WHERE id = $1 AND status = 'draft'"#,
            id,
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err("Job not found or not in Draft status".into());
        }

        Ok(())
    }
}
