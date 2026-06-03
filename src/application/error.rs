use thiserror::Error;

use crate::domain::pdf_validator::PdfValidationError;
use crate::domain::storage::StorageError;

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("email already registered")]
    EmailAlreadyRegistered,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("too many attempts")]
    TooManyAttempts { seconds_until_unlock: i64 },
    #[error("user not found")]
    UserNotFound,
    #[error(transparent)]
    Unexpected(#[from] DynError),
    #[error("user not active")]
    UserNotActive,
    #[error("invalid file: {0}")]
    InvalidFile(String),
    #[error("storage error: {0}")]
    StorageError(String),
    #[error("job not found")]
    JobNotFound,
    #[error("job is not in draft status")]
    JobNotDraft,
    #[error("job is not done yet")]
    JobNotDone,
}

#[derive(Debug)]
struct DisplayError(String);

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DisplayError {}

impl ApplicationError {
    pub fn from_display(err: impl std::fmt::Display) -> Self {
        Self::Unexpected(Box::new(DisplayError(err.to_string())))
    }
}

impl From<PdfValidationError> for ApplicationError {
    fn from(err: PdfValidationError) -> Self {
        use PdfValidationError::*;
        match err {
            EmptyFile => ApplicationError::InvalidFile("File is empty".into()),
            FileTooLarge {
                max_mb,
                actual_bytes,
            } => ApplicationError::InvalidFile(format!(
                "File too large (max {}MB, got {} bytes)",
                max_mb, actual_bytes
            )),
            InvalidMagicBytes => ApplicationError::InvalidFile("Not a valid PDF file".into()),
            CorruptOrUnreadable => {
                ApplicationError::InvalidFile("PDF is corrupt or unreadable".into())
            }
            PasswordProtected => ApplicationError::InvalidFile("PDF is password protected".into()),
        }
    }
}

impl From<StorageError> for ApplicationError {
    fn from(err: StorageError) -> Self {
        ApplicationError::StorageError(format!("{:?}", err))
    }
}
