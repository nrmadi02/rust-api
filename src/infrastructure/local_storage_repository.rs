use std::path::{Path, PathBuf};

use async_trait::async_trait;
use tokio::fs;
use uuid::Uuid;

use crate::domain::conversion_job::JobType;
use crate::domain::storage::{StorageError, StorageRepository, StorageResult, StoredPaths};

pub struct LocalStorageRepository {
    base_path: PathBuf,
    max_upload_bytes: u64,
}

impl LocalStorageRepository {
    pub fn new(base_path: impl Into<PathBuf>, max_upload_size_mb: u64) -> Self {
        Self {
            base_path: base_path.into(),
            max_upload_bytes: max_upload_size_mb * 1024 * 1024,
        }
    }

    fn uploads_root(&self) -> PathBuf {
        self.base_path.join("uploads")
    }

    fn outputs_root(&self) -> PathBuf {
        self.base_path.join("outputs")
    }

    fn input_dir(&self, user_id: Uuid, job_id: Uuid) -> PathBuf {
        self.uploads_root()
            .join(user_id.to_string())
            .join(job_id.to_string())
    }

    fn input_file(&self, user_id: Uuid, job_id: Uuid) -> PathBuf {
        self.input_dir(user_id, job_id).join("input.pdf")
    }

    fn output_dir(&self, job_id: Uuid) -> PathBuf {
        self.outputs_root().join(job_id.to_string())
    }

    fn output_filename(job_type: JobType) -> String {
        format!("output.{}", job_type.output_extension())
    }

    fn output_file(&self, job_id: Uuid, job_type: JobType) -> PathBuf {
        self.output_dir(job_id)
            .join(Self::output_filename(job_type))
    }

    fn check_size(&self, data: &[u8]) -> StorageResult<()> {
        let len = data.len() as u64;
        if len > self.max_upload_bytes {
            return Err(StorageError::FileTooLarge {
                max_mb: self.max_upload_bytes / (1024 * 1024),
                actual_bytes: len,
            });
        }
        Ok(())
    }

    async fn write_file(path: &Path, data: &[u8]) -> StorageResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        fs::write(path, data)
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(())
    }

    #[allow(dead_code)]
    fn relativize(&self, absolute: &Path) -> StorageResult<String> {
        absolute
            .strip_prefix(&self.base_path)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .map_err(|_| StorageError::InvalidPath)
    }
}

#[async_trait]
impl StorageRepository for LocalStorageRepository {
    fn input_relative_path(&self, user_id: Uuid, job_id: Uuid) -> String {
        format!("uploads/{}/{}/input.pdf", user_id, job_id)
    }

    fn output_relative_path(&self, job_id: Uuid, job_type: JobType) -> String {
        format!(
            "outputs/{}/{}",
            job_id,
            Self::output_filename(job_type)
        )
    }

    async fn ensure_layout(&self) -> StorageResult<()> {
        fs::create_dir_all(self.uploads_root())
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        fs::create_dir_all(self.outputs_root())
            .await
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(())
    }

    async fn save_input(
        &self,
        user_id: Uuid,
        job_id: Uuid,
        job_type: JobType,
        data: &[u8],
    ) -> StorageResult<StoredPaths> {
        self.check_size(data)?;
        let input = self.input_file(user_id, job_id);
        let output = self.output_file(job_id, job_type);
        Self::write_file(&input, data).await?;
        Ok(StoredPaths { input, output })
    }

    async fn save_output(
        &self,
        job_id: Uuid,
        job_type: JobType,
        data: &[u8],
    ) -> StorageResult<PathBuf> {
        let path = self.output_file(job_id, job_type);
        Self::write_file(&path, data).await?;
        Ok(path)
    }

    async fn read_input(&self, user_id: Uuid, job_id: Uuid) -> StorageResult<Vec<u8>> {
        let path = self.input_file(user_id, job_id);
        fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e.to_string())
            }
        })
    }

    async fn read_output(&self, job_id: Uuid, job_type: JobType) -> StorageResult<Vec<u8>> {
        let path = self.output_file(job_id, job_type);
        fs::read(&path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StorageError::NotFound
            } else {
                StorageError::Io(e.to_string())
            }
        })
    }

    async fn delete_job_files(&self, user_id: Uuid, job_id: Uuid) -> StorageResult<()> {
        let input_dir = self.input_dir(user_id, job_id);
        let output_dir = self.output_dir(job_id);

        if input_dir.exists() {
            fs::remove_dir_all(&input_dir)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        if output_dir.exists() {
            fs::remove_dir_all(&output_dir)
                .await
                .map_err(|e| StorageError::Io(e.to_string()))?;
        }
        Ok(())
    }
}
