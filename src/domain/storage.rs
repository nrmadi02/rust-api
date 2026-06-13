use std::path::PathBuf;
use uuid::Uuid;

use crate::domain::conversion_job::JobType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPaths {
    pub input: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    Io(String),
    FileTooLarge { max_mb: u64, actual_bytes: u64 },
    NotFound,
    InvalidPath,
}

pub type StorageResult<T> = Result<T, StorageError>;

#[async_trait::async_trait]
pub trait StorageRepository: Send + Sync {
    fn input_relative_path(&self, user_id: Uuid, job_id: Uuid, extension: &str) -> String;
    fn output_relative_path(&self, job_id: Uuid, job_type: JobType) -> String;
    fn image_dir_relative_path(&self, user_id: Uuid, job_id: Uuid) -> String;

    async fn ensure_layout(&self) -> StorageResult<()>;

    async fn save_input(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        job_type: JobType,
        extension: &str,
        data: &[u8],
    ) -> StorageResult<StoredPaths>;

    async fn save_image_input(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        index: usize,
        extension: &str,
        data: &[u8],
    ) -> StorageResult<()>;

    async fn save_output(
        &self,
        job_id: Uuid,
        job_type: JobType,
        data: &[u8],
    ) -> StorageResult<PathBuf>;

    async fn read_input(&self, user_id: Uuid, job_id: Uuid, extension: &str) -> StorageResult<Vec<u8>>;
    async fn read_output(&self, job_id: Uuid, job_type: JobType) -> StorageResult<Vec<u8>>;

    async fn delete_job_files(&self, user_id: Uuid, job_id: Uuid) -> StorageResult<()>;

    async fn save_page_images(
        &self,
        job_id: Uuid,
        pages: &[(u32, Vec<u8>)],
    ) -> StorageResult<()>;

    async fn read_page_image(&self, job_id: Uuid, page: u32) -> StorageResult<Vec<u8>>;
}
