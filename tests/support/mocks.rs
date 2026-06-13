use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

use task_tools::domain::activity_log::{ActivityLog, ActivityLogRepository};
use task_tools::domain::conversion_job::{
    ConversionJob, ConversionJobRepository, JobStatus, JobType, UnoConverter,
};
use task_tools::domain::image_to_pdf_converter::ImageToPdfConverter;
use task_tools::domain::image_validator::{ImageInfo, ImageValidationError, ImageValidator};
use task_tools::domain::pdf_to_image_converter::PdfToImageConverter;
use task_tools::domain::pdf_validator::{PdfInfo, PdfValidationError, PdfValidator};
use task_tools::domain::storage::{StorageError, StorageRepository, StorageResult, StoredPaths};
use task_tools::domain::word_validator::{
    WordFormat, WordInfo, WordValidationError, WordValidator,
};

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Default)]
pub struct MockJobRepo {
    inner: Mutex<MockJobRepoState>,
}

#[derive(Default)]
struct MockJobRepoState {
    jobs: HashMap<Uuid, ConversionJob>,
    delete_draft_calls: usize,
}

impl MockJobRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn seed(&self, job: ConversionJob) {
        self.inner.lock().await.jobs.insert(job.id, job);
    }

    pub async fn delete_draft_calls(&self) -> usize {
        self.inner.lock().await.delete_draft_calls
    }
}

#[async_trait]
impl ConversionJobRepository for MockJobRepo {
    async fn create_job(&self, job: &ConversionJob) -> Result<ConversionJob, DynError> {
        let mut state = self.inner.lock().await;
        state.jobs.insert(job.id, job.clone());
        Ok(job.clone())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ConversionJob>, DynError> {
        Ok(self.inner.lock().await.jobs.get(&id).cloned())
    }

    async fn find_by_user(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
        status: Option<JobStatus>,
    ) -> Result<(Vec<ConversionJob>, u64), DynError> {
        let jobs: Vec<_> = self
            .inner
            .lock()
            .await
            .jobs
            .values()
            .filter(|job| job.user_id == user_id)
            .filter(|job| status.is_none_or(|s| job.status == s))
            .cloned()
            .collect();

        let total = jobs.len() as u64;
        let start = ((page.saturating_sub(1)) * per_page) as usize;
        if start >= jobs.len() {
            return Ok((Vec::new(), total));
        }
        let end = (start + per_page as usize).min(jobs.len());
        Ok((jobs[start..end].to_vec(), total))
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: JobStatus,
        output_file: Option<&str>,
        error_message: Option<&str>,
        duration_ms: Option<i32>,
    ) -> Result<ConversionJob, DynError> {
        let mut state = self.inner.lock().await;
        let job = state
            .jobs
            .get_mut(&id)
            .ok_or_else(|| "job not found".to_string())?;
        job.status = status;
        if let Some(output) = output_file {
            job.output_file = Some(output.to_string());
        }
        if let Some(error) = error_message {
            job.error_message = Some(error.to_string());
        }
        if let Some(duration) = duration_ms {
            job.duration_ms = Some(duration);
        }
        Ok(job.clone())
    }

    async fn delete_draft(&self, id: Uuid) -> Result<(), DynError> {
        let mut state = self.inner.lock().await;
        state.delete_draft_calls += 1;
        state.jobs.remove(&id);
        Ok(())
    }
}

#[derive(Default)]
pub struct MockActivityLogRepo {
    logs: Mutex<Vec<ActivityLog>>,
}

impl MockActivityLogRepo {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn logs(&self) -> Vec<ActivityLog> {
        self.logs.lock().await.clone()
    }
}

#[async_trait]
impl ActivityLogRepository for MockActivityLogRepo {
    async fn log_activity(&self, log: &ActivityLog) -> Result<(), DynError> {
        self.logs.lock().await.push(log.clone());
        Ok(())
    }

    async fn find_by_user(
        &self,
        _user_id: Uuid,
        _page: u32,
        _per_page: u32,
        _action: Option<&str>,
    ) -> Result<(Vec<ActivityLog>, u64), DynError> {
        Ok((Vec::new(), 0))
    }
}

pub struct MockPdfValidator {
    pub info: PdfInfo,
}

impl MockPdfValidator {
    pub fn default_info() -> Self {
        Self {
            info: PdfInfo {
                page_count: 1,
                file_size_bytes: 1024,
                is_encrypted: false,
            },
        }
    }
}

impl PdfValidator for MockPdfValidator {
    fn validate(&self, bytes: &[u8]) -> Result<PdfInfo, PdfValidationError> {
        Ok(PdfInfo {
            page_count: self.info.page_count,
            file_size_bytes: bytes.len() as u64,
            is_encrypted: self.info.is_encrypted,
        })
    }
}

pub struct MockWordValidator;

impl WordValidator for MockWordValidator {
    fn validate(&self, bytes: &[u8], extension: &str) -> Result<WordInfo, WordValidationError> {
        let format = match extension.to_lowercase().as_str() {
            "docx" => WordFormat::Docx,
            "doc" => WordFormat::Doc,
            _ => return Err(WordValidationError::InvalidFormat),
        };
        Ok(WordInfo {
            file_size_bytes: bytes.len() as u64,
            file_format: format,
        })
    }
}

pub struct MockImageValidator;

impl ImageValidator for MockImageValidator {
    fn validate(&self, bytes: &[u8], extension: &str) -> Result<ImageInfo, ImageValidationError> {
        let ext = extension.to_lowercase();
        if !matches!(ext.as_str(), "jpg" | "jpeg" | "png") {
            return Err(ImageValidationError::InvalidFormat);
        }
        Ok(ImageInfo {
            file_size_bytes: bytes.len() as u64,
            width: 100,
            height: 100,
        })
    }
}

pub struct MockImageToPdfConverter;

impl ImageToPdfConverter for MockImageToPdfConverter {
    fn convert(&self, image_paths: &[&Path], output: &Path) -> Result<(), DynError> {
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            output,
            format!("mock-pdf-from-{}-images", image_paths.len()),
        )?;
        Ok(())
    }
}

pub struct MockPdfToImageConverter;

impl PdfToImageConverter for MockPdfToImageConverter {
    fn convert(&self, _input: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DynError> {
        std::fs::create_dir_all(output_dir)?;
        let page = output_dir.join("page-1.png");
        std::fs::write(&page, b"mock-png-data")?;
        Ok(vec![page])
    }
}

pub struct MockStorage {
    base_path: PathBuf,
}

impl MockStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
}

#[async_trait]
impl StorageRepository for MockStorage {
    fn input_relative_path(&self, user_id: Uuid, job_id: Uuid, extension: &str) -> String {
        format!("uploads/{user_id}/{job_id}/input.{}", extension)
    }

    fn output_relative_path(&self, job_id: Uuid, job_type: JobType) -> String {
        format!("outputs/{job_id}/output.{}", job_type.output_extension())
    }

    async fn ensure_layout(&self) -> StorageResult<()> {
        tokio::fs::create_dir_all(self.base_path.join("uploads"))
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        tokio::fs::create_dir_all(self.base_path.join("outputs"))
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(())
    }

    async fn save_input(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        _job_type: JobType,
        extension: &str,
        data: &[u8],
    ) -> StorageResult<StoredPaths> {
        let input = self
            .base_path
            .join(self.input_relative_path(user_id, job_id, extension));
        if let Some(parent) = input.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        tokio::fs::write(&input, data)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(StoredPaths {
            input,
            output: self.base_path.join("outputs"),
        })
    }

    fn image_dir_relative_path(&self, user_id: Uuid, job_id: Uuid) -> String {
        format!("uploads/{}/{}/images/", user_id, job_id)
    }

    async fn save_image_input(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        index: usize,
        extension: &str,
        data: &[u8],
    ) -> StorageResult<()> {
        let dir = self
            .base_path
            .join(self.image_dir_relative_path(user_id, job_id));
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        let path = dir.join(format!("page_{:03}.{}", index, extension));
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(())
    }

    async fn save_output(
        &self,
        job_id: Uuid,
        job_type: JobType,
        data: &[u8],
    ) -> StorageResult<PathBuf> {
        let output = self
            .base_path
            .join(self.output_relative_path(job_id, job_type));
        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        tokio::fs::write(&output, data)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(output)
    }

    async fn read_input(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        extension: &str,
    ) -> StorageResult<Vec<u8>> {
        let input = self
            .base_path
            .join(self.input_relative_path(user_id, job_id, extension));
        tokio::fs::read(input)
            .await
            .map_err(|_| StorageError::NotFound)
    }

    async fn read_output(&self, job_id: Uuid, job_type: JobType) -> StorageResult<Vec<u8>> {
        let output = self
            .base_path
            .join(self.output_relative_path(job_id, job_type));
        tokio::fs::read(output)
            .await
            .map_err(|_| StorageError::NotFound)
    }

    async fn delete_job_files(&self, user_id: Uuid, job_id: Uuid) -> StorageResult<()> {
        let input_dir = self
            .base_path
            .join("uploads")
            .join(user_id.to_string())
            .join(job_id.to_string());
        if input_dir.exists() {
            tokio::fs::remove_dir_all(input_dir)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        Ok(())
    }

    async fn save_page_images(&self, job_id: Uuid, pages: &[(u32, Vec<u8>)]) -> StorageResult<()> {
        let dir = self
            .base_path
            .join("outputs")
            .join(job_id.to_string())
            .join("pages");
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        for (page_num, data) in pages {
            let path = dir.join(format!("page_{:03}.png", page_num));
            tokio::fs::write(&path, data)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        Ok(())
    }

    async fn read_page_image(&self, job_id: Uuid, page: u32) -> StorageResult<Vec<u8>> {
        let path = self
            .base_path
            .join("outputs")
            .join(job_id.to_string())
            .join("pages")
            .join(format!("page_{:03}.png", page));
        tokio::fs::read(&path)
            .await
            .map_err(|_| StorageError::NotFound)
    }
}

pub struct MockUnoConverter;

#[async_trait]
impl UnoConverter for MockUnoConverter {
    async fn convert(
        &self,
        _input: &Path,
        output: &Path,
        _job_type: &JobType,
    ) -> Result<(), DynError> {
        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(output, b"mock-docx-output").await?;
        Ok(())
    }
}

pub fn conversion_service(
    job_repo: Arc<MockJobRepo>,
    activity_log_repo: Arc<MockActivityLogRepo>,
    storage: Arc<MockStorage>,
    base_path: PathBuf,
) -> task_tools::application::conversion::ConversionService {
    task_tools::application::conversion::ConversionService::new(
        job_repo,
        activity_log_repo,
        storage,
        task_tools::application::conversion::FileValidators {
            pdf: Arc::new(MockPdfValidator::default_info()),
            word: Arc::new(MockWordValidator),
            image: Arc::new(MockImageValidator),
        },
        task_tools::application::conversion::Converters {
            uno: Arc::new(MockUnoConverter),
            image_to_pdf: Arc::new(MockImageToPdfConverter),
            pdf_to_image: Arc::new(MockPdfToImageConverter),
        },
        base_path,
    )
}
