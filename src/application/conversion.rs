use std::sync::Arc;

use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::domain::activity_log::{ActivityLog, ActivityLogRepository};
use crate::domain::conversion_job::{
    ConversionJob, ConversionJobRepository, JobStatus, JobType, UnoConverter,
};
use crate::domain::image_to_pdf_converter::ImageToPdfConverter;
use crate::domain::image_validator::ImageValidator;
use crate::domain::pdf_to_image_converter::PdfToImageConverter;
use crate::domain::pdf_validator::PdfValidator;
use crate::domain::storage::StorageRepository;
use crate::domain::word_validator::WordValidator;
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

pub struct FileValidators {
    pub pdf: Arc<dyn PdfValidator>,
    pub word: Arc<dyn WordValidator>,
    pub image: Arc<dyn ImageValidator>,
}

pub struct Converters {
    pub uno: Arc<dyn UnoConverter>,
    pub image_to_pdf: Arc<dyn ImageToPdfConverter>,
    pub pdf_to_image: Arc<dyn PdfToImageConverter>,
}

pub struct ConversionService {
    job_repo: Arc<dyn ConversionJobRepository>,
    activity_log_repo: Arc<dyn ActivityLogRepository>,
    storage: Arc<dyn StorageRepository>,
    validators: FileValidators,
    converters: Converters,
    storage_base_path: PathBuf,
}

impl ConversionService {
    pub fn new(
        job_repo: Arc<dyn ConversionJobRepository>,
        activity_log_repo: Arc<dyn ActivityLogRepository>,
        storage: Arc<dyn StorageRepository>,
        validators: FileValidators,
        converters: Converters,
        storage_base_path: PathBuf,
    ) -> Self {
        Self {
            job_repo,
            activity_log_repo,
            storage,
            validators,
            converters,
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
        let pdf_info = self.validators.pdf.validate(file_bytes)?;
        let job_id = Uuid::new_v4();
        let input_path = self
            .storage
            .input_relative_path(user_id, job_id, &extension);
        let job = ConversionJob::new_with_id(job_id, user_id, JobType::PdfToWord, input_path);
        self.storage
            .save_input(user_id, job_id, JobType::PdfToWord, &extension, file_bytes)
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

    pub async fn upload_word_to_pdf(
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
        if !matches!(extension.as_str(), "doc" | "docx") {
            return Err(ApplicationError::InvalidFile(
                "Only DOC/DOCX files are accepted".into(),
            ));
        }
        let word_info = self.validators.word.validate(file_bytes, &extension)?;
        let job_id = Uuid::new_v4();
        let input_path = self
            .storage
            .input_relative_path(user_id, job_id, &extension);
        let job = ConversionJob::new_with_id(job_id, user_id, JobType::WordToPdf, input_path);
        self.storage
            .save_input(user_id, job_id, JobType::WordToPdf, &extension, file_bytes)
            .await?;
        let saved_job = self.job_repo.create_job(&job).await?;

        let activity = ActivityLog::upload_word_file(
            user_id,
            saved_job.id,
            original_filename,
            word_info.file_size_bytes as i64,
        );
        self.activity_log_repo.log_activity(&activity).await?;
        Ok(UploadResult { job: saved_job })
    }

    pub async fn upload_image_to_pdf(
        &self,
        user_id: Uuid,
        files: &[(Vec<u8>, String)],
        order: Option<&[usize]>,
    ) -> Result<UploadResult, ApplicationError> {
        if files.is_empty() {
            return Err(ApplicationError::InvalidFile(
                "At least one image is required".into(),
            ));
        }
        if files.len() > 50 {
            return Err(ApplicationError::InvalidFile(
                "Maximum 50 images per upload".into(),
            ));
        }

        let ordered_indices = if let Some(ord) = order {
            if ord.len() != files.len() {
                return Err(ApplicationError::InvalidFile(
                    "Order length must match number of files".into(),
                ));
            }
            ord.to_vec()
        } else {
            (0..files.len()).collect()
        };

        let job_id = Uuid::new_v4();
        let mut total_size: i64 = 0;

        for &idx in &ordered_indices {
            let (file_bytes, original_name) = &files[idx];
            let extension = original_name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            let _ = self.validators.image.validate(file_bytes, &extension)?;
            total_size += file_bytes.len() as i64;
        }

        for (seq, &idx) in ordered_indices.iter().enumerate() {
            let (file_bytes, original_name) = &files[idx];
            let extension = original_name
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            self.storage
                .save_image_input(user_id, job_id, seq, &extension, file_bytes)
                .await?;
        }

        let input_path = self.storage.image_dir_relative_path(user_id, job_id);
        let job = ConversionJob::new_with_id(job_id, user_id, JobType::ImageToPdf, input_path);
        let saved_job = self.job_repo.create_job(&job).await?;

        let activity = ActivityLog::upload_images(user_id, saved_job.id, files.len(), total_size);
        self.activity_log_repo.log_activity(&activity).await?;
        Ok(UploadResult { job: saved_job })
    }

    pub async fn upload_pdf_to_image(
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
        let pdf_info = self.validators.pdf.validate(file_bytes)?;
        let job_id = Uuid::new_v4();
        let input_path = self
            .storage
            .input_relative_path(user_id, job_id, &extension);
        let job = ConversionJob::new_with_id(job_id, user_id, JobType::PdfToImage, input_path);
        self.storage
            .save_input(user_id, job_id, JobType::PdfToImage, &extension, file_bytes)
            .await?;
        let saved_job = self.job_repo.create_job(&job).await?;

        let activity = ActivityLog::upload_pdf_to_image(
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

        let activity = ActivityLog::confirm_job(user_id, job_id);
        if let Err(err) = self.activity_log_repo.log_activity(&activity).await {
            log::error!("Failed to log confirm for job {}: {}", job_id, err);
        }

        let worker = ConversionWorker::new(
            self.job_repo.clone(),
            self.storage.clone(),
            self.converters.uno.clone(),
            self.converters.image_to_pdf.clone(),
            self.converters.pdf_to_image.clone(),
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
            JobType::WordToPdf | JobType::ImageToPdf => "application/pdf",
            JobType::PdfToImage => "application/zip",
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

    pub async fn download_page(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        page: u32,
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

        let bytes = self.storage.read_page_image(job.id, page).await?;
        let file_name = format!("page_{:03}.png", page);

        let activity = ActivityLog::download_file(user_id, job.id, &file_name);
        if let Err(err) = self.activity_log_repo.log_activity(&activity).await {
            log::error!("Failed to log download for job {}: {}", job.id, err);
        }

        Ok(DownloadConvertedFileResult {
            bytes,
            file_name,
            content_type: "image/png".to_string(),
        })
    }
}
