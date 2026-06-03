use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use crate::domain::conversion_job::{JobType, UnoConverter};

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone, Copy)]
pub enum ConvertFormat {
    Docx,
    Pdf,
}

impl ConvertFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ConvertFormat::Docx => "docx",
            ConvertFormat::Pdf => "pdf",
        }
    }

    pub fn input_filter(self) -> Option<&'static str> {
        match self {
            ConvertFormat::Docx => Some("writer_pdf_import"),
            ConvertFormat::Pdf => None,
        }
    }

    pub fn from_job_type(job_type: &JobType) -> Self {
        match job_type {
            JobType::PdfToWord => ConvertFormat::Docx,
            JobType::WordToPdf => ConvertFormat::Pdf,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("failed to spawn unoconvert: {0}")]
    Spawn(std::io::Error),

    #[error("failed to access unoconvert stdin")]
    MissingStdin,

    #[error("failed to write input bytes to unoconvert stdin: {0}")]
    WriteStdin(std::io::Error),

    #[error("failed to wait for unoconvert output: {0}")]
    Wait(std::io::Error),

    #[error("unoconvert timed out after {0} seconds")]
    Timeout(u64),

    #[error("unoconvert exited with status {status}: {stderr}")]
    ProcessFailed { status: String, stderr: String },
}

pub struct UnoserverClient {
    host: String,
    port: u16,
    timeout_secs: u64,
}

impl UnoserverClient {
    pub fn new(host: String, port: u16, timeout_secs: u64) -> Self {
        Self {
            host,
            port,
            timeout_secs,
        }
    }

    pub async fn convert(
        &self,
        input_bytes: Bytes,
        format: ConvertFormat,
    ) -> Result<Bytes, ConvertError> {
        match self.convert_once(input_bytes.clone(), format).await {
            Ok(output) => Ok(output),
            Err(first_error) => {
                log::warn!("unoconvert failed, retrying once: {}", first_error);
                self.convert_once(input_bytes, format).await
            }
        }
    }

    async fn convert_once(
        &self,
        input_bytes: Bytes,
        format: ConvertFormat,
    ) -> Result<Bytes, ConvertError> {
        let timeout_secs = self.timeout_secs;

        timeout(
            Duration::from_secs(timeout_secs),
            self.run_unoconvert(input_bytes, format),
        )
        .await
        .map_err(|_| ConvertError::Timeout(timeout_secs))?
    }

    async fn run_unoconvert(
        &self,
        input_bytes: Bytes,
        format: ConvertFormat,
    ) -> Result<Bytes, ConvertError> {
        let port = self.port.to_string();

        let mut command = Command::new("unoconvert");

        command.args([
            "--host",
            self.host.as_str(),
            "--port",
            port.as_str(),
            "--convert-to",
            format.as_str(),
        ]);

        if let Some(input_filter) = format.input_filter() {
            command.args(["--input-filter", input_filter]);
        }

        command
            .args(["-", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(ConvertError::Spawn)?;

        let mut stdin = child.stdin.take().ok_or(ConvertError::MissingStdin)?;
        stdin
            .write_all(&input_bytes)
            .await
            .map_err(ConvertError::WriteStdin)?;

        drop(stdin);

        let output = child.wait_with_output().await.map_err(ConvertError::Wait)?;

        if !output.status.success() {
            let status = output.status.to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            return Err(ConvertError::ProcessFailed { status, stderr });
        }

        Ok(Bytes::from(output.stdout))
    }
}

#[async_trait]
impl UnoConverter for UnoserverClient {
    async fn convert(
        &self,
        input: &Path,
        output: &Path,
        job_type: &JobType,
    ) -> Result<(), DynError> {
        let input_bytes = tokio::fs::read(input).await?;
        let format = ConvertFormat::from_job_type(job_type);

        let output_bytes = self.convert(Bytes::from(input_bytes), format).await?;

        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(output, &output_bytes).await?;

        Ok(())
    }
}
