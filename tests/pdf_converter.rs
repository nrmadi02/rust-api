mod support;

use std::time::{Duration, Instant};

use bytes::Bytes;
use support::fixtures::{corrupt_pdf_bytes, minimal_valid_pdf, not_a_pdf_bytes};
use task_tools::domain::pdf_validator::{PdfValidationError, PdfValidator};
use task_tools::infrastructure::pdf_validator::LopPdfValidator;
use task_tools::infrastructure::unoserver_client::{ConvertError, ConvertFormat, UnoserverClient};

#[test]
fn validator_accepts_minimal_valid_pdf() {
    let validator = LopPdfValidator::new(50);
    let info = validator
        .validate(&minimal_valid_pdf())
        .expect("valid pdf should pass");

    assert!(info.page_count >= 1);
    assert!(!info.is_encrypted);
}

#[test]
fn validator_rejects_non_pdf_magic_bytes() {
    let validator = LopPdfValidator::new(50);
    let err = validator
        .validate(&not_a_pdf_bytes())
        .expect_err("plain text should fail");

    assert_eq!(err, PdfValidationError::InvalidMagicBytes);
}

#[test]
fn validator_rejects_corrupt_pdf() {
    let validator = LopPdfValidator::new(50);
    let err = validator
        .validate(&corrupt_pdf_bytes())
        .expect_err("broken pdf should fail");

    assert_eq!(err, PdfValidationError::CorruptOrUnreadable);
}

#[test]
fn validator_rejects_empty_file() {
    let validator = LopPdfValidator::new(50);
    let err = validator.validate(&[]).expect_err("empty file should fail");
    assert_eq!(err, PdfValidationError::EmptyFile);
}

#[tokio::test]
async fn converter_returns_error_for_invalid_pdf_input() {
    let client = UnoserverClient::new("127.0.0.1".to_string(), 2003, 5);
    let result = client
        .convert(Bytes::from(not_a_pdf_bytes()), ConvertFormat::Docx)
        .await;

    assert!(result.is_err(), "non-pdf input should not produce docx");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ConvertError::ProcessFailed { .. } | ConvertError::Spawn(_) | ConvertError::Timeout(_)
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn converter_enforces_timeout() {
    let client = UnoserverClient::new("127.0.0.1".to_string(), 1, 1);
    let started = Instant::now();
    let result = client
        .convert(Bytes::from(minimal_valid_pdf()), ConvertFormat::Docx)
        .await;
    let elapsed = started.elapsed();

    assert!(result.is_err(), "unreachable unoserver should fail");
    assert!(
        elapsed < Duration::from_secs(10),
        "expected quick failure, took {:?}",
        elapsed
    );

    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ConvertError::Timeout(_) | ConvertError::ProcessFailed { .. } | ConvertError::Spawn(_)
        ),
        "unexpected error: {err}"
    );
}

#[tokio::test]
#[ignore = "requires unoserver on UNOSERVER_HOST:UNOSERVER_PORT and unoconvert in PATH"]
async fn converter_converts_valid_pdf_to_docx() {
    dotenvy::dotenv().ok();

    let host = std::env::var("UNOSERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("UNOSERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2003);
    let timeout = std::env::var("UNOSERVER_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);

    let client = UnoserverClient::new(host, port, timeout);
    let output = client
        .convert(Bytes::from(minimal_valid_pdf()), ConvertFormat::Docx)
        .await
        .expect("conversion should succeed with running unoserver");

    assert!(!output.is_empty(), "docx output should not be empty");
    assert!(
        output.starts_with(b"PK"),
        "docx is a zip archive and should start with PK bytes"
    );
}
