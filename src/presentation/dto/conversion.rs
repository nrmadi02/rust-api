use serde::Serialize;
use utoipa::ToSchema;

use crate::domain::conversion_job::{ConversionJob, JobStatus, JobType};

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversionJobResponse {
    pub id: uuid::Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub input_file: String,
    pub output_file: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ConversionJob> for ConversionJobResponse {
    fn from(job: ConversionJob) -> Self {
        Self {
            id: job.id,
            job_type: job.job_type,
            status: job.status,
            input_file: job.input_file,
            output_file: job.output_file,
            error_message: job.error_message,
            duration_ms: job.duration_ms,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}
