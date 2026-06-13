use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::domain::conversion_job::{
    ConversionJob, ConversionJobRepository, JobStatus, JobType, UnoConverter,
};
use crate::domain::image_to_pdf_converter::ImageToPdfConverter;
use crate::domain::pdf_to_image_converter::PdfToImageConverter;
use crate::domain::storage::StorageRepository;
use std::io::Write;
use zip::write::SimpleFileOptions;

#[derive(Clone)]
pub struct ConversionWorker {
    job_repo: Arc<dyn ConversionJobRepository>,
    storage: Arc<dyn StorageRepository>,
    uno_converter: Arc<dyn UnoConverter>,
    image_to_pdf_converter: Arc<dyn ImageToPdfConverter>,
    pdf_to_image_converter: Arc<dyn PdfToImageConverter>,
    storage_base_path: PathBuf,
}

impl ConversionWorker {
    pub fn new(
        job_repo: Arc<dyn ConversionJobRepository>,
        storage: Arc<dyn StorageRepository>,
        uno_converter: Arc<dyn UnoConverter>,
        image_to_pdf_converter: Arc<dyn ImageToPdfConverter>,
        pdf_to_image_converter: Arc<dyn PdfToImageConverter>,
        storage_base_path: PathBuf,
    ) -> Self {
        Self {
            job_repo,
            storage,
            uno_converter,
            image_to_pdf_converter,
            pdf_to_image_converter,
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

        let output_relative = self.storage.output_relative_path(job.id, job.job_type);
        let output_path = self.storage_base_path.join(&output_relative);

        let result = match job.job_type {
            JobType::PdfToWord | JobType::WordToPdf => {
                let input_path = self.storage_base_path.join(&job.input_file);
                self.uno_converter
                    .convert(&input_path, &output_path, &job.job_type)
                    .await
            }
            JobType::ImageToPdf => {
                let image_dir = self.storage_base_path.join(&job.input_file);
                self.convert_images(&image_dir, &output_path)
            }
            JobType::PdfToImage => {
                let input_path = self.storage_base_path.join(&job.input_file);
                self.convert_pdf_to_images(&input_path, job.id, &output_path)
            }
        };

        match result {
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

    fn convert_images(
        &self,
        image_dir: &Path,
        output: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut entries: Vec<_> = std::fs::read_dir(image_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| matches!(ext.to_str(), Some("jpg" | "jpeg" | "png")))
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        let paths: Vec<_> = entries.iter().map(|e| e.path()).collect();
        let refs: Vec<_> = paths.iter().map(|p| p.as_path()).collect();

        let converter = self.image_to_pdf_converter.clone();
        let output = output.to_path_buf();
        tokio::task::block_in_place(|| converter.convert(&refs, &output))
    }

    fn convert_pdf_to_images(
        &self,
        input: &Path,
        job_id: uuid::Uuid,
        output_zip: &Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let converter = self.pdf_to_image_converter.clone();
        let render_dir = output_zip
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("render_tmp");
        let render_dir_clone = render_dir.clone();

        let images = tokio::task::block_in_place(|| converter.convert(input, &render_dir_clone))?;

        let pages: Vec<(u32, Vec<u8>)> = images
            .iter()
            .enumerate()
            .map(|(i, path)| {
                let data = std::fs::read(path)?;
                Ok::<(u32, Vec<u8>), std::io::Error>(((i + 1) as u32, data))
            })
            .collect::<Result<_, _>>()?;

        let storage = self.storage.clone();
        let pages_ref: Vec<(u32, Vec<u8>)> = pages.clone();
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(storage.save_page_images(job_id, &pages_ref))
                .map_err(|e| format!("{:?}", e))
        })?;

        let zip_bytes = Self::create_zip(&pages)?;

        if let Some(parent) = output_zip.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(output_zip, &zip_bytes)?;

        std::fs::remove_dir_all(&render_dir).ok();

        Ok(())
    }

    fn create_zip(
        pages: &[(u32, Vec<u8>)],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip_writer = zip::ZipWriter::new(&mut buf);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        for (page_num, data) in pages {
            let name = format!("page_{:03}.png", page_num);
            zip_writer.start_file(&name, options)?;
            zip_writer.write_all(data)?;
        }

        zip_writer.finish()?;
        Ok(buf.into_inner())
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
