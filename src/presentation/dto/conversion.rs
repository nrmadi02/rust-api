use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::domain::conversion_job::{ConversionJob, JobStatus, JobType};
use crate::presentation::response::api::PaginationMeta;

#[derive(Debug, ToSchema)]
pub struct UploadFileRequest {
    #[schema(value_type = String)]
    pub file: String,
}

#[derive(Debug, ToSchema)]
pub struct UploadImagesRequest {
    #[schema(value_type = Vec<String>)]
    pub files: Vec<String>,

    #[schema(value_type = Option<String>, example = "0,2,1")]
    pub order: Option<String>,
}

fn deserialize_optional_job_status<'de, D>(deserializer: D) -> Result<Option<JobStatus>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        None => Ok(None),
        Some(val) => {
            let status = match val.to_lowercase().as_str() {
                "draft" => JobStatus::Draft,
                "processing" => JobStatus::Processing,
                "queued" => JobStatus::Queued,
                "done" => JobStatus::Done,
                "failed" => JobStatus::Failed,
                other => {
                    return Err(serde::de::Error::custom(format!(
                        "invalid status '{}', valid values: draft, processing, queued, done, failed",
                        other
                    )));
                }
            };
            Ok(Some(status))
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListJobsQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    #[param(
        value_type = Option<JobStatus>,
        example = "done",
        nullable = true
    )]
    #[serde(default, deserialize_with = "deserialize_optional_job_status")]
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
