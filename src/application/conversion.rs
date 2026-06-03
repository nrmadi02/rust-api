use std::sync::Arc;

use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::domain::activity_log::{ActivityLog, ActivityLogRepository};
use crate::domain::conversion_job::{
    ConversionJob, ConversionJobRepository, JobStatus, JobType, UnoConverter,
};
use crate::domain::pdf_validator::PdfValidator;
use crate::domain::storage::StorageRepository;
use std::path::PathBuf;

use crate::application::conversion_worker::ConversionWorker;

#[derive(Debug)]
pub struct UploadResult {
    pub job: ConversionJob,
}

#[derive(Debug)]
pub struct DownloadConvertedFileResult {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub content_type: String,
}

pub struct ConversionService {
    job_repo: Arc<dyn ConversionJobRepository>,
    activity_log_repo: Arc<dyn ActivityLogRepository>,
    storage: Arc<dyn StorageRepository>,
    pdf_validator: Arc<dyn PdfValidator>,
    uno_converter: Arc<dyn UnoConverter>,
    storage_base_path: PathBuf,
}

impl ConversionService {
    pub fn new(
        job_repo: Arc<dyn ConversionJobRepository>,
        activity_log_repo: Arc<dyn ActivityLogRepository>,
        storage: Arc<dyn StorageRepository>,
        pdf_validator: Arc<dyn PdfValidator>,
        uno_converter: Arc<dyn UnoConverter>,
        storage_base_path: PathBuf,
    ) -> Self {
        Self {
            job_repo,
            activity_log_repo,
            storage,
            pdf_validator,
            uno_converter,
            storage_base_path,
        }
    }

    pub async fn upload_pdf_to_word(
        &self,
        user_id: Uuid,
        file_bytes: &[u8],
        original_filename: &str,
    ) -> Result<UploadResult, ApplicationError> {
        let extension = original_filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if extension != "pdf" {
            return Err(ApplicationError::InvalidFile(
                "Only PDF files are accepted".into(),
            ));
        }
        let pdf_info = self.pdf_validator.validate(file_bytes)?;
        let job_id = Uuid::new_v4();
        let input_path = self.storage.input_relative_path(user_id, job_id);
        let job = ConversionJob::new_with_id(job_id, user_id, JobType::PdfToWord, input_path);
        self.storage
            .save_input(user_id, job_id, JobType::PdfToWord, file_bytes)
            .await?;
        let saved_job = self.job_repo.create_job(&job).await?;

        let activity = ActivityLog::upload_file(
            user_id,
            saved_job.id,
            original_filename,
            pdf_info.file_size_bytes as i64,
            pdf_info.page_count,
        );
        self.activity_log_repo.log_activity(&activity).await?;
        Ok(UploadResult { job: saved_job })
    }

    pub async fn list_my_conversion_jobs(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
        status: Option<JobStatus>,
    ) -> Result<(Vec<ConversionJob>, u64), ApplicationError> {
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);

        let result = self
            .job_repo
            .find_by_user(user_id, page, per_page, status)
            .await?;

        Ok(result)
    }

    pub async fn get_conversion_job_status(
        &self,
        job_id: Uuid,
        user_id: Uuid,
    ) -> Result<ConversionJob, ApplicationError> {
        let job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or(ApplicationError::JobNotFound)?;

        if job.user_id != user_id {
            return Err(ApplicationError::JobNotFound);
        }

        Ok(job)
    }

    pub async fn delete_draft_job(
        &self,
        job_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), ApplicationError> {
        let job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or(ApplicationError::JobNotFound)?;

        if job.user_id != user_id {
            return Err(ApplicationError::JobNotFound);
        }

        if job.status != JobStatus::Draft {
            return Err(ApplicationError::JobNotDraft);
        }

        self.storage.delete_job_files(user_id, job_id).await?;
        self.job_repo.delete_draft(job_id).await?;

        let activity = ActivityLog::delete_job(user_id, job_id);
        if let Err(err) = self.activity_log_repo.log_activity(&activity).await {
            log::error!("Failed to log deletion for job {}: {}", job_id, err);
        }

        Ok(())
    }

    pub async fn enqueue_conversion_job(
        &self,
        job_id: Uuid,
        user_id: Uuid,
    ) -> Result<ConversionJob, ApplicationError> {
        let mut job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or(ApplicationError::JobNotFound)?;

        if job.user_id != user_id {
            return Err(ApplicationError::JobNotFound);
        }

        if !job.status.can_enqueue() {
            return Err(ApplicationError::JobNotDraft);
        }
        job.enqueue().map_err(ApplicationError::from_display)?;

        let queued_job = self
            .job_repo
            .update_status(job.id, JobStatus::Queued, None, None, None)
            .await?;

        let worker = ConversionWorker::new(
            self.job_repo.clone(),
            self.storage.clone(),
            self.uno_converter.clone(),
            self.storage_base_path.clone(),
        );

        worker.spawn(queued_job.clone());

        Ok(queued_job)
    }

    pub async fn download_converted_file(
        &self,
        user_id: Uuid,
        job_id: Uuid,
    ) -> Result<DownloadConvertedFileResult, ApplicationError> {
        let job = self
            .job_repo
            .find_by_id(job_id)
            .await?
            .ok_or(ApplicationError::JobNotFound)?;

        if job.user_id != user_id {
            return Err(ApplicationError::JobNotFound);
        }

        if job.status != JobStatus::Done {
            return Err(ApplicationError::JobNotDone);
        }

        if job.output_file.is_none() {
            return Err(ApplicationError::StorageError(
                "Output file is missing".to_string(),
            ));
        }

        let bytes = self.storage.read_output(job.id, job.job_type).await?;

        let file_name = format!("converted-{}.{}", job.id, job.job_type.output_extension());

        let content_type = match job.job_type {
            JobType::PdfToWord => {
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            }
            JobType::WordToPdf => "application/pdf",
        }
        .to_string();

        let activity = ActivityLog::download_file(user_id, job.id, &file_name);

        if let Err(err) = self.activity_log_repo.log_activity(&activity).await {
            log::error!("Failed to log download for job {}: {}", job.id, err);
        }

        Ok(DownloadConvertedFileResult {
            bytes,
            file_name,
            content_type,
        })
    }
}
