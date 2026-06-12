use task_tools::presentation::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn openapi_documents_all_conversion_and_activity_endpoints() {
    let doc = ApiDoc::openapi();
    let paths: Vec<String> = doc.paths.paths.keys().cloned().collect();

    let expected = [
        "/api/v1/convert/pdf-to-word",
        "/api/v1/convert/jobs",
        "/api/v1/convert/jobs/{id}",
        "/api/v1/convert/jobs/{id}/confirm",
        "/api/v1/convert/jobs/{id}/download",
        "/api/v1/me/activity-logs",
    ];

    for endpoint in expected {
        assert!(
            paths.iter().any(|path| path == endpoint),
            "missing OpenAPI path: {endpoint}"
        );
    }
}

#[test]
fn openapi_list_jobs_documents_status_query_param() {
    let doc = ApiDoc::openapi();
    let list_jobs = doc
        .paths
        .paths
        .get("/api/v1/convert/jobs")
        .expect("list jobs path");
    let get = list_jobs.get.as_ref().expect("GET operation");
    let param_names: Vec<String> = get
        .parameters
        .iter()
        .flat_map(|group| group.iter())
        .map(|param| param.name.clone())
        .collect();

    assert!(
        param_names.iter().any(|name| name == "status"),
        "expected status query param in OpenAPI, got: {param_names:?}"
    );
}
