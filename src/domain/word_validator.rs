#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordInfo {
    pub file_size_bytes: u64,
    pub file_format: WordFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordFormat {
    Doc,
    Docx,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordValidationError {
    EmptyFile,
    FileTooLarge { max_mb: u64, actual_bytes: u64 },
    InvalidFormat,
}

pub trait WordValidator: Send + Sync {
    fn validate(&self, bytes: &[u8], extension: &str) -> Result<WordInfo, WordValidationError>;
}
