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

        let pdf_info = inspect_pdf(bytes).map_err(|_| PdfValidationError::CorruptOrUnreadable)?;

        if pdf_info.is_encrypted {
            return Err(PdfValidationError::PasswordProtected);
        }

        Ok(pdf_info)
    }
}

fn has_pdf_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF")
}

fn inspect_pdf(bytes: &[u8]) -> Result<PdfInfo, lopdf::Error> {
    let doc = Document::load_mem(bytes)?;

    Ok(PdfInfo {
        page_count: doc.get_pages().len(),
        file_size_bytes: bytes.len() as u64,
        is_encrypted: doc.is_encrypted(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::pdf_validator::PdfValidationError;
    use lopdf::{dictionary, Document, Object};

    fn minimal_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        });
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).expect("serialize pdf");
        buffer
    }

    #[test]
    fn accepts_valid_pdf() {
        let validator = LopPdfValidator::new(50);
        let info = validator.validate(&minimal_pdf()).expect("valid pdf");
        assert!(info.page_count >= 1);
    }

    #[test]
    fn rejects_invalid_magic_bytes() {
        let validator = LopPdfValidator::new(50);
        let err = validator
            .validate(b"hello")
            .expect_err("non-pdf should fail");
        assert_eq!(err, PdfValidationError::InvalidMagicBytes);
    }

    #[test]
    fn rejects_corrupt_pdf() {
        let validator = LopPdfValidator::new(50);
        let err = validator
            .validate(b"%PDF-1.4\nbroken")
            .expect_err("corrupt pdf should fail");
        assert_eq!(err, PdfValidationError::CorruptOrUnreadable);
    }
}
