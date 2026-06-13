use std::path::Path;

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub trait ImageToPdfConverter: Send + Sync {
    fn convert(&self, image_paths: &[&Path], output: &Path) -> Result<(), DynError>;
}
