use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::conversion_job::{ConversionJob, JobStatus, JobType};
use crate::presentation::response::api::PaginationMeta;

#[derive(Debug, ToSchema)]
pub struct UploadFileRequest {
    #[schema(value_type = String)]
    pub file: String,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListJobsQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status: Option<JobStatus>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversionJobResponse {
    pub id: uuid::Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub download_url: Option<String>,
    pub input_file: String,
    pub output_file: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ConversionJob> for ConversionJobResponse {
    fn from(job: ConversionJob) -> Self {
        let download_url = (job.status == JobStatus::Done)
            .then(|| format!("/api/v1/convert/jobs/{}/download", job.id));

        Self {
            id: job.id,
            job_type: job.job_type,
            status: job.status,
            download_url,
            input_file: job.input_file,
            output_file: job.output_file,
            error_message: job.error_message,
            duration_ms: job.duration_ms,
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListJobsResponse {
    pub items: Vec<ConversionJobResponse>,
    pub pagination: PaginationMeta,
}
