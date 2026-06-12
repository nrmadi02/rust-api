mod support;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use support::app::setup_test_app;
use support::fixtures::{minimal_valid_pdf, multipart_pdf_body};
use task_tools::domain::conversion_job::JobType;
use task_tools::domain::storage::StorageRepository;
use task_tools::infrastructure::local_storage_repository::LocalStorageRepository;
use tower::ServiceExt;
use uuid::Uuid;

async fn body_to_json(body: Body) -> serde_json::Value {
    let bytes = body.collect().await.expect("body bytes").to_bytes();
    serde_json::from_slice(&bytes).expect("valid json body")
}

#[tokio::test]
async fn upload_pdf_without_token_returns_401() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    }

    let app = setup_test_app().await;
    let (content_type, body) = multipart_pdf_body("sample.pdf", &minimal_valid_pdf());

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/convert/pdf-to-word")
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn upload_valid_pdf_creates_draft_job() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    }

    let app = setup_test_app().await;
    let (content_type, body) = multipart_pdf_body("sample.pdf", &minimal_valid_pdf());

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/convert/pdf-to-word")
                .header("authorization", format!("Bearer {}", app.token))
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::ACCEPTED);

    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "draft");
    assert!(json["data"]["id"].is_string());
}

#[tokio::test]
async fn list_jobs_returns_only_current_user_jobs() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    }

    let app = setup_test_app().await;
    let (content_type, body) = multipart_pdf_body("sample.pdf", &minimal_valid_pdf());

    app.router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/convert/pdf-to-word")
                .header("authorization", format!("Bearer {}", app.token))
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("upload response");

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/convert/jobs")
                .header("authorization", format!("Bearer {}", app.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list response");

    assert_eq!(response.status(), StatusCode::OK);

    let json = body_to_json(response.into_body()).await;
    assert_eq!(json["success"], true);
    assert!(!json["data"]["items"].as_array().unwrap().is_empty());
    assert!(json["data"]["pagination"]["total"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn list_jobs_filters_by_status_query_param() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    }

    let app = setup_test_app().await;
    let (content_type, body) = multipart_pdf_body("sample.pdf", &minimal_valid_pdf());

    let upload_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/convert/pdf-to-word")
                .header("authorization", format!("Bearer {}", app.token))
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("upload response");

    let upload_json = body_to_json(upload_response.into_body()).await;
    let job_id = upload_json["data"]["id"]
        .as_str()
        .expect("job id")
        .parse::<Uuid>()
        .expect("valid uuid");

    sqlx::query(
        r#"
        UPDATE conversion_jobs
        SET status = 'done', output_file = $1
        WHERE id = $2
        "#,
    )
    .bind(format!("outputs/{job_id}/output.docx"))
    .bind(job_id)
    .execute(&app.pool)
    .await
    .expect("mark job done");

    let all_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/convert/jobs")
                .header("authorization", format!("Bearer {}", app.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list all response");
    let all_json = body_to_json(all_response.into_body()).await;
    assert_eq!(all_json["data"]["pagination"]["total"], 1);

    let draft_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/convert/jobs?status=draft")
                .header("authorization", format!("Bearer {}", app.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list draft response");
    let draft_json = body_to_json(draft_response.into_body()).await;
    assert_eq!(draft_json["data"]["pagination"]["total"], 0);

    let done_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/convert/jobs?status=done")
                .header("authorization", format!("Bearer {}", app.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list done response");
    let done_json = body_to_json(done_response.into_body()).await;
    assert_eq!(done_json["data"]["pagination"]["total"], 1);
    assert_eq!(done_json["data"]["items"][0]["status"], "done");

    let invalid_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/convert/jobs?status=sembarang")
                .header("authorization", format!("Bearer {}", app.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("list invalid response");
    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn download_done_job_returns_file() {
    if std::env::var("DATABASE_URL").is_err() {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    }

    let app = setup_test_app().await;
    let (content_type, body) = multipart_pdf_body("sample.pdf", &minimal_valid_pdf());

    let upload_response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/convert/pdf-to-word")
                .header("authorization", format!("Bearer {}", app.token))
                .header("content-type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .expect("upload response");

    let upload_json = body_to_json(upload_response.into_body()).await;
    let job_id = upload_json["data"]["id"]
        .as_str()
        .expect("job id")
        .parse::<Uuid>()
        .expect("valid uuid");

    let output_path = format!("outputs/{job_id}/output.docx");
    sqlx::query(
        r#"
        UPDATE conversion_jobs
        SET status = 'done', output_file = $1
        WHERE id = $2
        "#,
    )
    .bind(&output_path)
    .bind(job_id)
    .execute(&app.pool)
    .await
    .expect("mark job done");

    let storage = LocalStorageRepository::new(app.storage_dir.path(), 50);
    storage
        .save_output(job_id, JobType::PdfToWord, b"fake-docx-content")
        .await
        .expect("save output");

    let response = app
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/convert/jobs/{job_id}/download"))
                .header("authorization", format!("Bearer {}", app.token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("download response");

    if response.status() != StatusCode::OK {
        let error_body = body_to_json(response.into_body()).await;
        panic!("download failed: {error_body}");
    }

    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
    );

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("download body")
        .to_bytes();
    assert_eq!(bytes.as_ref(), b"fake-docx-content");
}
