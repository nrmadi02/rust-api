#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfInfo {
    pub page_count: usize,
    pub file_size_bytes: u64,
    pub is_encrypted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfValidationError {
    EmptyFile,
    FileTooLarge { max_mb: u64, actual_bytes: u64 },
    InvalidMagicBytes,
    CorruptOrUnreadable,
    PasswordProtected,
}

pub trait PdfValidator: Send + Sync {
    fn validate(&self, bytes: &[u8]) -> Result<PdfInfo, PdfValidationError>;
}
