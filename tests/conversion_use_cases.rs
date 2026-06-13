mod support;

use std::sync::Arc;

use support::fixtures::minimal_valid_pdf;
use support::mocks::{MockActivityLogRepo, MockJobRepo, MockStorage, conversion_service};
use task_tools::application::error::ApplicationError;
use task_tools::domain::conversion_job::{ConversionJob, JobStatus, JobType};
use task_tools::domain::storage::StorageRepository;
use uuid::Uuid;

#[tokio::test]
async fn upload_pdf_to_word_creates_draft_job_and_logs_activity() {
    let temp = tempfile::tempdir().expect("tempdir");
    let job_repo = Arc::new(MockJobRepo::new());
    let activity_repo = Arc::new(MockActivityLogRepo::new());
    let storage = Arc::new(MockStorage::new(temp.path().to_path_buf()));
    storage.ensure_layout().await.expect("storage layout");

    let service = conversion_service(
        job_repo.clone(),
        activity_repo.clone(),
        storage,
        temp.path().to_path_buf(),
    );

    let user_id = Uuid::new_v4();
    let pdf = minimal_valid_pdf();

    let result = service
        .upload_pdf_to_word(user_id, &pdf, "sample.pdf")
        .await
        .expect("upload should succeed");

    assert_eq!(result.job.status, JobStatus::Draft);
    assert_eq!(result.job.job_type, JobType::PdfToWord);
    assert_eq!(result.job.user_id, user_id);

    let logs = activity_repo.logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].action, "upload_pdf");
}

#[tokio::test]
async fn upload_rejects_non_pdf_extension() {
    let temp = tempfile::tempdir().expect("tempdir");
    let service = conversion_service(
        Arc::new(MockJobRepo::new()),
        Arc::new(MockActivityLogRepo::new()),
        Arc::new(MockStorage::new(temp.path().to_path_buf())),
        temp.path().to_path_buf(),
    );

    let err = service
        .upload_pdf_to_word(Uuid::new_v4(), b"%PDF-1.4", "notes.txt")
        .await
        .expect_err("txt upload should fail");

    assert!(matches!(err, ApplicationError::InvalidFile(_)));
}

#[tokio::test]
async fn get_conversion_job_status_enforces_ownership() {
    let temp = tempfile::tempdir().expect("tempdir");
    let job_repo = Arc::new(MockJobRepo::new());
    let owner_id = Uuid::new_v4();
    let other_user = Uuid::new_v4();
    let job = ConversionJob::new(owner_id, JobType::PdfToWord, "uploads/input.pdf".into());
    job_repo.seed(job.clone()).await;

    let service = conversion_service(
        job_repo,
        Arc::new(MockActivityLogRepo::new()),
        Arc::new(MockStorage::new(temp.path().to_path_buf())),
        temp.path().to_path_buf(),
    );

    let owned = service
        .get_conversion_job_status(job.id, owner_id)
        .await
        .expect("owner should access job");
    assert_eq!(owned.id, job.id);

    let err = service
        .get_conversion_job_status(job.id, other_user)
        .await
        .expect_err("other user should not access job");
    assert!(matches!(err, ApplicationError::JobNotFound));
}

#[tokio::test]
async fn delete_draft_job_only_allows_draft_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let job_repo = Arc::new(MockJobRepo::new());
    let user_id = Uuid::new_v4();

    let draft = ConversionJob::new(user_id, JobType::PdfToWord, "uploads/input.pdf".into());
    job_repo.seed(draft.clone()).await;

    let mut queued = ConversionJob::new(user_id, JobType::PdfToWord, "uploads/input2.pdf".into());
    queued.enqueue().expect("enqueue");
    job_repo.seed(queued.clone()).await;

    let storage = Arc::new(MockStorage::new(temp.path().to_path_buf()));
    storage.ensure_layout().await.expect("storage layout");
    storage
        .save_input(user_id, draft.id, JobType::PdfToWord, "pdf", &minimal_valid_pdf())
        .await
        .expect("save draft input");

    let service = conversion_service(
        job_repo.clone(),
        Arc::new(MockActivityLogRepo::new()),
        storage,
        temp.path().to_path_buf(),
    );

    service
        .delete_draft_job(draft.id, user_id)
        .await
        .expect("draft delete should succeed");
    assert_eq!(job_repo.delete_draft_calls().await, 1);

    let err = service
        .delete_draft_job(queued.id, user_id)
        .await
        .expect_err("queued job should not be deletable");
    assert!(matches!(err, ApplicationError::JobNotDraft));
}

#[tokio::test]
async fn list_jobs_returns_only_current_user_jobs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let job_repo = Arc::new(MockJobRepo::new());
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();

    job_repo
        .seed(ConversionJob::new(user_a, JobType::PdfToWord, "a1.pdf".into()))
        .await;
    job_repo
        .seed(ConversionJob::new(user_a, JobType::PdfToWord, "a2.pdf".into()))
        .await;
    job_repo
        .seed(ConversionJob::new(user_b, JobType::PdfToWord, "b1.pdf".into()))
        .await;

    let service = conversion_service(
        job_repo,
        Arc::new(MockActivityLogRepo::new()),
        Arc::new(MockStorage::new(temp.path().to_path_buf())),
        temp.path().to_path_buf(),
    );

    let (jobs, total) = service
        .list_my_conversion_jobs(user_a, 1, 10, None)
        .await
        .expect("list should succeed");

    assert_eq!(total, 2);
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|j| j.user_id == user_a));
}

#[tokio::test]
async fn list_jobs_filters_by_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let job_repo = Arc::new(MockJobRepo::new());
    let user_id = Uuid::new_v4();

    let draft = ConversionJob::new(user_id, JobType::PdfToWord, "draft.pdf".into());
    let mut queued = ConversionJob::new(user_id, JobType::PdfToWord, "queued.pdf".into());
    queued.enqueue().expect("enqueue");

    job_repo.seed(draft.clone()).await;
    job_repo.seed(queued.clone()).await;

    let service = conversion_service(
        job_repo,
        Arc::new(MockActivityLogRepo::new()),
        Arc::new(MockStorage::new(temp.path().to_path_buf())),
        temp.path().to_path_buf(),
    );

    let (draft_jobs, draft_total) = service
        .list_my_conversion_jobs(user_id, 1, 10, Some(JobStatus::Draft))
        .await
        .expect("list draft should succeed");
    assert_eq!(draft_total, 1);
    assert_eq!(draft_jobs[0].id, draft.id);

    let (queued_jobs, queued_total) = service
        .list_my_conversion_jobs(user_id, 1, 10, Some(JobStatus::Queued))
        .await
        .expect("list queued should succeed");
    assert_eq!(queued_total, 1);
    assert_eq!(queued_jobs[0].id, queued.id);

    let (_, done_total) = service
        .list_my_conversion_jobs(user_id, 1, 10, Some(JobStatus::Done))
        .await
        .expect("list done should succeed");
    assert_eq!(done_total, 0);
}

#[tokio::test]
async fn list_jobs_paginates_correctly() {
    let temp = tempfile::tempdir().expect("tempdir");
    let job_repo = Arc::new(MockJobRepo::new());
    let user_id = Uuid::new_v4();

    for i in 0..5 {
        job_repo
            .seed(ConversionJob::new(
                user_id,
                JobType::PdfToWord,
                format!("file{i}.pdf"),
            ))
            .await;
    }

    let service = conversion_service(
        job_repo,
        Arc::new(MockActivityLogRepo::new()),
        Arc::new(MockStorage::new(temp.path().to_path_buf())),
        temp.path().to_path_buf(),
    );

    let (page1, total) = service
        .list_my_conversion_jobs(user_id, 1, 2, None)
        .await
        .expect("page 1 should succeed");
    assert_eq!(total, 5);
    assert_eq!(page1.len(), 2);

    let (page3, _) = service
        .list_my_conversion_jobs(user_id, 3, 2, None)
        .await
        .expect("page 3 should succeed");
    assert_eq!(page3.len(), 1);

    let (beyond, _) = service
        .list_my_conversion_jobs(user_id, 99, 2, None)
        .await
        .expect("out of range page should return empty");
    assert!(beyond.is_empty());
}
