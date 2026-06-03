use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crate::domain::conversion_job::{
    ConversionJob, ConversionJobRepository, JobStatus, UnoConverter,
};
use crate::domain::storage::StorageRepository;

#[derive(Clone)]
pub struct ConversionWorker {
    job_repo: Arc<dyn ConversionJobRepository>,
    storage: Arc<dyn StorageRepository>,
    uno_converter: Arc<dyn UnoConverter>,
    storage_base_path: PathBuf,
}

impl ConversionWorker {
    pub fn new(
        job_repo: Arc<dyn ConversionJobRepository>,
        storage: Arc<dyn StorageRepository>,
        uno_converter: Arc<dyn UnoConverter>,
        storage_base_path: PathBuf,
    ) -> Self {
        Self {
            job_repo,
            storage,
            uno_converter,
            storage_base_path,
        }
    }

    pub fn spawn(&self, job: ConversionJob) {
        let worker = self.clone();
        let panic_handler = self.clone();
        let job_id = job.id;
        let start = Instant::now();

        let handle = tokio::spawn(async move {
            worker.run(job, start).await;
        });

        tokio::spawn(async move {
            if let Err(join_error) = handle.await {
                let duration_ms = start.elapsed().as_millis() as i32;
                let error_message = if join_error.is_panic() {
                    "conversion worker panicked".to_string()
                } else {
                    format!("conversion worker cancelled: {}", join_error)
                };

                panic_handler
                    .mark_failed(job_id, &error_message, Some(duration_ms))
                    .await;
            }
        });
    }

    async fn run(&self, job: ConversionJob, start: Instant) {
        if let Err(error) = self
            .job_repo
            .update_status(job.id, JobStatus::Processing, None, None, None)
            .await
        {
            log::error!("Failed to set job {} to processing: {}", job.id, error);
            return;
        }

        let input_path = self.storage_base_path.join(&job.input_file);
        let output_relative = self.storage.output_relative_path(job.id, job.job_type);
        let output_path = self.storage_base_path.join(&output_relative);

        match self
            .uno_converter
            .convert(&input_path, &output_path, &job.job_type)
            .await
        {
            Ok(()) => {
                let duration_ms = start.elapsed().as_millis() as i32;

                if let Err(error) = self
                    .job_repo
                    .update_status(
                        job.id,
                        JobStatus::Done,
                        Some(&output_relative),
                        None,
                        Some(duration_ms),
                    )
                    .await
                {
                    log::error!("Failed to mark job {} done: {}", job.id, error);
                }
            }
            Err(error) => {
                let duration_ms = start.elapsed().as_millis() as i32;
                let error_message = error.to_string();

                self.mark_failed(job.id, &error_message, Some(duration_ms))
                    .await;
            }
        }
    }

    async fn mark_failed(&self, job_id: uuid::Uuid, error_message: &str, duration_ms: Option<i32>) {
        log::error!("Job {} failed: {}", job_id, error_message);

        if let Err(error) = self
            .job_repo
            .update_status(
                job_id,
                JobStatus::Failed,
                None,
                Some(error_message),
                duration_ms,
            )
            .await
        {
            log::error!("Failed to mark job {} failed: {}", job_id, error);
        }
    }
}
