use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum JobStatus {
    Draft,
    Processing,
    Done,
    Failed,
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, JobStatus::Done | JobStatus::Failed)
    }

    pub fn can_process(&self) -> bool {
        matches!(self, JobStatus::Draft)
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
            created_at: now,
            updated_at: now,
        }
    }
    pub fn start_processing(&mut self) -> Result<(), String> {
        if !self.status.can_process() {
            return Err(format!(
                "Cannot start processing job with status {:?}",
                self.status
            ));
        }
        self.status = JobStatus::Processing;
        self.updated_at = Utc::now();
        Ok(())
    }
    pub fn mark_done(&mut self, output_file: String) {
        self.status = JobStatus::Done;
        self.output_file = Some(output_file);
        self.error_message = None;
        self.updated_at = Utc::now();
    }

    pub fn mark_failed(&mut self, error_message: String) {
        self.status = JobStatus::Failed;
        self.error_message = Some(error_message);
        self.updated_at = Utc::now();
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
