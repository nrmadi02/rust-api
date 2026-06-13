use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::domain::pdf_to_image_converter::PdfToImageConverter;

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum PdfToImageError {
    #[error("failed to spawn pdftoppm: {0}")]
    Spawn(std::io::Error),

    #[error("pdftoppm exited with status {status}: {stderr}")]
    ProcessFailed { status: String, stderr: String },

    #[error("pdftoppm produced no output images")]
    NoOutput,
}

pub struct PopplerPdfToImageConverter {
    dpi: u32,
}

impl PopplerPdfToImageConverter {
    pub fn new(dpi: u32) -> Self {
        Self { dpi }
    }
}

impl PdfToImageConverter for PopplerPdfToImageConverter {
    fn convert(&self, input: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DynError> {
        std::fs::create_dir_all(output_dir)?;

        let output_prefix = output_dir.join("page");

        let status = Command::new("pdftoppm")
            .args([
                "-png",
                "-r",
                self.dpi.to_string().as_str(),
                "-cropbox",
            ])
            .arg(input)
            .arg(&output_prefix)
            .output()
            .map_err(PdfToImageError::Spawn)?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr).to_string();
            return Err(PdfToImageError::ProcessFailed {
                status: status.status.to_string(),
                stderr,
            }
            .into());
        }

        let mut images: Vec<PathBuf> = std::fs::read_dir(output_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|ext| ext == "png")
            })
            .map(|e| e.path())
            .collect();

        images.sort();

        if images.is_empty() {
            return Err(PdfToImageError::NoOutput.into());
        }

        Ok(images)
    }
}
