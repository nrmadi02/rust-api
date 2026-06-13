use crate::domain::word_validator::{WordFormat, WordInfo, WordValidationError, WordValidator};

pub struct SimpleWordValidator {
    max_upload_size_mb: u64,
}

impl SimpleWordValidator {
    pub fn new(max_upload_size_mb: u64) -> Self {
        Self { max_upload_size_mb }
    }
}

impl WordValidator for SimpleWordValidator {
    fn validate(&self, bytes: &[u8], extension: &str) -> Result<WordInfo, WordValidationError> {
        if bytes.is_empty() {
            return Err(WordValidationError::EmptyFile);
        }

        let file_size_bytes = bytes.len() as u64;
        let max_bytes = self.max_upload_size_mb * 1024 * 1024;

        if file_size_bytes > max_bytes {
            return Err(WordValidationError::FileTooLarge {
                max_mb: self.max_upload_size_mb,
                actual_bytes: file_size_bytes,
            });
        }

        let ext_lower = extension.to_lowercase();
        let format = match ext_lower.as_str() {
            "docx" => {
                if !is_zip_magic(bytes) {
                    return Err(WordValidationError::InvalidFormat);
                }
                WordFormat::Docx
            }
            "doc" => {
                if !is_ole_magic(bytes) {
                    return Err(WordValidationError::InvalidFormat);
                }
                WordFormat::Doc
            }
            _ => return Err(WordValidationError::InvalidFormat),
        };

        Ok(WordInfo {
            file_size_bytes,
            file_format: format,
        })
    }
}

fn is_zip_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == [0x50, 0x4B, 0x03, 0x04]
}

fn is_ole_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 8
        && bytes[0..8] == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_docx_magic() {
        let validator = SimpleWordValidator::new(50);
        let mut docx = vec![0x50, 0x4B, 0x03, 0x04];
        docx.extend_from_slice(b"rest of file content here");

        let info = validator.validate(&docx, "docx").expect("valid docx");
        assert_eq!(info.file_format, WordFormat::Docx);
        assert_eq!(info.file_size_bytes, docx.len() as u64);
    }

    #[test]
    fn accepts_valid_doc_magic() {
        let validator = SimpleWordValidator::new(50);
        let mut doc = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
        doc.extend_from_slice(b"rest of file");

        let info = validator.validate(&doc, "doc").expect("valid doc");
        assert_eq!(info.file_format, WordFormat::Doc);
    }

    #[test]
    fn rejects_empty_file() {
        let validator = SimpleWordValidator::new(50);
        let err = validator.validate(&[], "docx").expect_err("empty");
        assert_eq!(err, WordValidationError::EmptyFile);
    }

    #[test]
    fn rejects_invalid_magic_bytes() {
        let validator = SimpleWordValidator::new(50);
        let err = validator.validate(b"hello world", "docx").expect_err("bad magic");
        assert_eq!(err, WordValidationError::InvalidFormat);
    }

    #[test]
    fn rejects_file_too_large() {
        let validator = SimpleWordValidator::new(1);
        let big = vec![0x50, 0x4B, 0x03, 0x04];
        let big = vec![big; 1_000_000].concat();

        let err = validator.validate(&big, "docx").expect_err("too large");
        assert!(matches!(err, WordValidationError::FileTooLarge { .. }));
    }

    #[test]
    fn rejects_wrong_extension() {
        let validator = SimpleWordValidator::new(50);
        let err = validator.validate(b"some data", "txt").expect_err("bad ext");
        assert_eq!(err, WordValidationError::InvalidFormat);
    }
}
