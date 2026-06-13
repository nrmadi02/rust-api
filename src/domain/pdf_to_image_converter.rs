use std::path::{Path, PathBuf};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub trait PdfToImageConverter: Send + Sync {
    fn convert(&self, input: &Path, output_dir: &Path) -> Result<Vec<PathBuf>, DynError>;
}
