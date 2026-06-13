#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub file_size_bytes: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageValidationError {
    EmptyFile,
    FileTooLarge { max_mb: u64, actual_bytes: u64 },
    InvalidFormat,
    DecodeFailed(String),
}

pub trait ImageValidator: Send + Sync {
    fn validate(&self, bytes: &[u8], extension: &str) -> Result<ImageInfo, ImageValidationError>;
}
