use crate::domain::image_validator::{ImageInfo, ImageValidationError, ImageValidator};

pub struct SimpleImageValidator {
    max_upload_size_mb: u64,
}

impl SimpleImageValidator {
    pub fn new(max_upload_size_mb: u64) -> Self {
        Self { max_upload_size_mb }
    }
}

impl ImageValidator for SimpleImageValidator {
    fn validate(&self, bytes: &[u8], extension: &str) -> Result<ImageInfo, ImageValidationError> {
        if bytes.is_empty() {
            return Err(ImageValidationError::EmptyFile);
        }

        let file_size_bytes = bytes.len() as u64;
        let max_bytes = self.max_upload_size_mb * 1024 * 1024;

        if file_size_bytes > max_bytes {
            return Err(ImageValidationError::FileTooLarge {
                max_mb: self.max_upload_size_mb,
                actual_bytes: file_size_bytes,
            });
        }

        let ext_lower = extension.to_lowercase();
        if !matches!(ext_lower.as_str(), "jpg" | "jpeg" | "png") {
            return Err(ImageValidationError::InvalidFormat);
        }

        if !is_valid_image_magic(bytes, &ext_lower) {
            return Err(ImageValidationError::InvalidFormat);
        }

        let img = image::load_from_memory(bytes)
            .map_err(|e| ImageValidationError::DecodeFailed(e.to_string()))?;

        Ok(ImageInfo {
            file_size_bytes,
            width: img.width(),
            height: img.height(),
        })
    }
}

fn is_valid_image_magic(bytes: &[u8], extension: &str) -> bool {
    match extension {
        "jpg" | "jpeg" => bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF,
        "png" => bytes.len() >= 8 && bytes[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_png() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(2, 2, image::Rgb([255, 0, 0]));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        buf
    }

    fn create_minimal_jpeg() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(2, 2, image::Rgb([0, 255, 0]));
        let mut buf = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
            .unwrap();
        buf
    }

    #[test]
    fn accepts_valid_png() {
        let validator = SimpleImageValidator::new(50);
        let png = create_minimal_png();
        let info = validator.validate(&png, "png").expect("valid png");
        assert_eq!(info.width, 2);
        assert_eq!(info.height, 2);
    }

    #[test]
    fn accepts_valid_jpeg() {
        let validator = SimpleImageValidator::new(50);
        let jpeg = create_minimal_jpeg();
        let info = validator.validate(&jpeg, "jpg").expect("valid jpg");
        assert_eq!(info.width, 2);
    }

    #[test]
    fn accepts_valid_jpeg_extension() {
        let validator = SimpleImageValidator::new(50);
        let jpeg = create_minimal_jpeg();
        let info = validator.validate(&jpeg, "jpeg").expect("valid jpeg");
        assert_eq!(info.width, 2);
    }

    #[test]
    fn rejects_empty_file() {
        let validator = SimpleImageValidator::new(50);
        let err = validator.validate(&[], "png").expect_err("empty");
        assert_eq!(err, ImageValidationError::EmptyFile);
    }

    #[test]
    fn rejects_invalid_magic() {
        let validator = SimpleImageValidator::new(50);
        let err = validator.validate(b"hello world", "png").expect_err("bad");
        assert_eq!(err, ImageValidationError::InvalidFormat);
    }

    #[test]
    fn rejects_wrong_extension() {
        let validator = SimpleImageValidator::new(50);
        let err = validator.validate(b"data", "gif").expect_err("bad ext");
        assert_eq!(err, ImageValidationError::InvalidFormat);
    }
}
