use lopdf::Document;

use crate::domain::pdf_validator::{PdfInfo, PdfValidationError, PdfValidator};

pub struct LopPdfValidator {
    max_upload_size_mb: u64,
}

impl LopPdfValidator {
    pub fn new(max_upload_size_mb: u64) -> Self {
        Self { max_upload_size_mb }
    }

    fn max_upload_bytes(&self) -> u64 {
        self.max_upload_size_mb * 1024 * 1024
    }
}

impl PdfValidator for LopPdfValidator {
    fn validate(&self, bytes: &[u8]) -> Result<PdfInfo, PdfValidationError> {
        if bytes.is_empty() {
            return Err(PdfValidationError::EmptyFile);
        }

        let file_size_bytes = bytes.len() as u64;
        let max_bytes = self.max_upload_bytes();
        if file_size_bytes > max_bytes {
            return Err(PdfValidationError::FileTooLarge {
                max_mb: self.max_upload_size_mb,
                actual_bytes: file_size_bytes,
            });
        }

        if !has_pdf_magic(bytes) {
            return Err(PdfValidationError::InvalidMagicBytes);
        }

        let (page_count, is_encrypted) =
            inspect_pdf(bytes).map_err(|_| PdfValidationError::CorruptOrUnreadable)?;

        if is_encrypted {
            return Err(PdfValidationError::PasswordProtected);
        }

        Ok(PdfInfo {
            page_count,
            file_size_bytes,
            is_encrypted,
        })
    }
}

fn has_pdf_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF")
}

fn inspect_pdf(bytes: &[u8]) -> Result<(usize, bool), lopdf::Error> {
    let doc = Document::load_mem(bytes)?;
    let page_count = doc.get_pages().len();
    let is_encrypted = doc.is_encrypted();
    Ok((page_count, is_encrypted))
}
