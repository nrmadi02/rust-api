use std::path::Path;

use async_trait::async_trait;
use reqwest::Client;

use crate::domain::conversion_job::{JobType, UnoConverter};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct UnoserverClient {
    host: String,
    port: u16,
    client: Client,
}

impl UnoserverClient {
    pub fn new(host: String, port: u16, timeout_secs: u64) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client");
        Self { host, port, client }
    }
}

#[async_trait]
impl UnoConverter for UnoserverClient {
    async fn convert(
        &self,
        input: &Path,
        output: &Path,
        _job_type: &JobType,
    ) -> Result<(), DynError> {
        let url = format!("http://{}:{}/request", self.host, self.port);

        let input_bytes = tokio::fs::read(input).await?;
        let input_filename = input
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let part = reqwest::multipart::Part::bytes(input_bytes)
            .file_name(input_filename)
            .mime_str("application/octet-stream")?;

        let output_ext = output
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_else(|| "pdf".to_string());

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("convert-to", output_ext);

        let response = self.client.post(&url).multipart(form).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Unoserver error {}: {}", status, body).into());
        }

        let output_bytes = response.bytes().await?;
        if let Some(parent) = output.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(output, &output_bytes).await?;

        Ok(())
    }
}
