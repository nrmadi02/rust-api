mod support;

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use bytes::Bytes;
use support::fixtures::{create_sized_pdf, minimal_valid_pdf};
use task_tools::infrastructure::unoserver_client::{ConvertFormat, UnoserverClient};

const MB: usize = 1024 * 1024;

#[derive(Debug)]
struct BenchmarkResult {
    label: String,
    input_bytes: usize,
    duration_ms: u128,
    output_bytes: usize,
    success: bool,
    error: Option<String>,
}

fn write_benchmark_report(results: &[BenchmarkResult], path: &PathBuf) {
    let mut lines = vec![
        "# PDF to DOCX Conversion Benchmarks".to_string(),
        String::new(),
        format!("> Generated: {}", chrono::Utc::now().to_rfc3339()),
        String::new(),
        "| File Size | Input (bytes) | Duration (ms) | Output (bytes) | Status |".to_string(),
        "|-----------|---------------|---------------|----------------|--------|".to_string(),
    ];

    for result in results {
        let status = if result.success {
            "OK".to_string()
        } else {
            result.error.clone().unwrap_or_else(|| "FAILED".to_string())
        };

        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            result.label, result.input_bytes, result.duration_ms, result.output_bytes, status
        ));
    }

    lines.push(String::new());
    lines.push("## How to run".to_string());
    lines.push(String::new());
    lines.push("```bash".to_string());
    lines.push("# Start unoserver container first".to_string());
    lines.push("cargo test --test performance_benchmark -- --ignored --nocapture".to_string());
    lines.push("```".to_string());

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create docs directory");
    }
    fs::write(path, lines.join("\n")).expect("write benchmark report");
}

async fn run_single_benchmark(
    client: &UnoserverClient,
    label: &str,
    pdf_bytes: Vec<u8>,
) -> BenchmarkResult {
    let input_bytes = pdf_bytes.len();
    let started = Instant::now();

    match client
        .convert(Bytes::from(pdf_bytes), ConvertFormat::Docx)
        .await
    {
        Ok(output) => BenchmarkResult {
            label: label.to_string(),
            input_bytes,
            duration_ms: started.elapsed().as_millis(),
            output_bytes: output.len(),
            success: true,
            error: None,
        },
        Err(err) => BenchmarkResult {
            label: label.to_string(),
            input_bytes,
            duration_ms: started.elapsed().as_millis(),
            output_bytes: 0,
            success: false,
            error: Some(err.to_string()),
        },
    }
}

#[tokio::test]
#[ignore = "manual performance benchmark — requires unoserver and unoconvert"]
async fn benchmark_pdf_conversion_sizes() {
    dotenvy::dotenv().ok();

    let host = std::env::var("UNOSERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("UNOSERVER_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2003);
    let timeout = std::env::var("UNOSERVER_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);

    let client = UnoserverClient::new(host, port, timeout);

    let targets = [("~1 MB", MB), ("~10 MB", 10 * MB), ("~50 MB", 50 * MB)];

    let mut results = Vec::new();

    for (label, size) in targets {
        let pdf = if size <= MB {
            let bytes = minimal_valid_pdf();
            if bytes.len() < size {
                create_sized_pdf(size)
            } else {
                bytes
            }
        } else {
            create_sized_pdf(size)
        };

        println!("running benchmark {label}: input={} bytes", pdf.len());

        let result = run_single_benchmark(&client, label, pdf).await;
        println!("{label}: {:?}", result);
        results.push(result);
    }

    let report_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/benchmarks.md");
    write_benchmark_report(&results, &report_path);

    assert!(
        results.iter().any(|r| r.success),
        "at least one benchmark size should succeed when unoserver is available"
    );
}
