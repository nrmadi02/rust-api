use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::path::Path;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text")]
pub enum JobType {
    PdfToWord,
    WordToPdf,
}

impl JobType {
    pub fn output_extension(&self) -> &str {
        match self {
            JobType::PdfToWord => "docx",
            JobType::WordToPdf => "pdf",
        }
    }

    pub fn is_valid_input(&self, extension: &str) -> bool {
        match self {
            JobType::PdfToWord => extension.eq_ignore_ascii_case("pdf"),
            JobType::WordToPdf => matches!(extension.to_lowercase().as_str(), "doc" | "docx"),
        }
    }

    pub fn default_input_extension(&self) -> &str {
        match self {
            JobType::PdfToWord => "pdf",
            JobType::WordToPdf => "docx",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum JobStatus {
    Draft,
    Processing,
    Queued,
    Done,
    Failed,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Draft => "draft",
            JobStatus::Processing => "processing",
            JobStatus::Queued => "queued",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Done | JobStatus::Failed)
    }

    pub fn can_enqueue(&self) -> bool {
        matches!(self, JobStatus::Draft)
    }

    pub fn can_process(&self) -> bool {
        matches!(self, JobStatus::Queued)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ConversionJob {
    pub id: Uuid,
    pub user_id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub input_file: String,
    pub output_file: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ConversionJob {
    pub fn new(user_id: Uuid, job_type: JobType, input_file: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            job_type,
            status: JobStatus::Draft,
            input_file,
            output_file: None,
            error_message: None,
            duration_ms: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_with_id(id: Uuid, user_id: Uuid, job_type: JobType, input_file: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            job_type,
            status: JobStatus::Draft,
            input_file,
            output_file: None,
            error_message: None,
            duration_ms: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn enqueue(&mut self) -> Result<(), String> {
        if !self.status.can_enqueue() {
            return Err(format!("Cannot enqueue job with status {:?}", self.status));
        }
        self.status = JobStatus::Queued;
        self.updated_at = Utc::now();
        Ok(())
    }

    pub fn validate_input_file(&self) -> Result<(), String> {
        let extension = self
            .input_file
            .split('.')
            .next_back()
            .ok_or("File has no extension")?;

        if !self.job_type.is_valid_input(extension) {
            return Err(format!(
                "Invalid input file extension '{}' for job type {:?}",
                extension, self.job_type
            ));
        }

        Ok(())
    }
    pub fn generate_output_filename(&self) -> String {
        let input_stem = self
            .input_file
            .split('.')
            .next()
            .unwrap_or(&self.input_file);
        format!(
            "{}_converted.{}",
            input_stem,
            self.job_type.output_extension()
        )
    }
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait ConversionJobRepository: Send + Sync {
    async fn create_job(&self, job: &ConversionJob) -> Result<ConversionJob, DynError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ConversionJob>, DynError>;
    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
        status: Option<JobStatus>,
    ) -> Result<(Vec<ConversionJob>, u64), DynError>;
    async fn update_status(
        &self,
        id: Uuid,
        status: JobStatus,
        output_file: Option<&str>,
        error_message: Option<&str>,
        duration_ms: Option<i32>,
    ) -> Result<ConversionJob, DynError>;
    async fn delete_draft(&self, id: Uuid) -> Result<(), DynError>;
}

#[async_trait::async_trait]
pub trait UnoConverter: Send + Sync {
    async fn convert(
        &self,
        input: &Path,
        output: &Path,
        job_type: &JobType,
    ) -> Result<(), DynError>;
}
