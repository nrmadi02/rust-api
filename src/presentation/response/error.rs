use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

use crate::application::error::ApplicationError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("An unexpected error occurred")]
    InternalServerError,
    #[error("Resource not found")]
    NotFound,
    #[error("Bad request")]
    BadRequest,
    #[error("Authentication required")]
    Unauthorized,
    #[error("Access forbidden")]
    Forbidden,
    #[error("Validation failed")]
    Validation(Vec<String>),
    #[error("{message}")]
    Custom {
        status: StatusCode,
        code: &'static str,
        message: String,
    },
}

impl AppError {
    pub fn custom(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self::Custom {
            status,
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<String>>,
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Custom { status, .. } => *status,
        }
    }
    fn code(&self) -> &'static str {
        match self {
            AppError::InternalServerError => "INTERNAL_SERVER_ERROR",
            AppError::NotFound => "NOT_FOUND",
            AppError::BadRequest => "BAD_REQUEST",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::Validation(_) => "VALIDATION_ERROR",
            AppError::Custom { code, .. } => code,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code();
        let message = self.to_string();

        let details = match self {
            AppError::Validation(details) => Some(details.clone()),
            _ => None,
        };

        let body = ErrorResponse {
            success: false,
            error: ErrorDetail {
                code: code.to_string(),
                message,
                details,
            },
        };
        (status, Json(body)).into_response()
    }
}

impl From<ApplicationError> for AppError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::EmailAlreadyRegistered => AppError::custom(
                StatusCode::CONFLICT,
                "EMAIL_ALREADY_REGISTERED",
                "Email is already registered",
            ),
            ApplicationError::InvalidCredentials => AppError::custom(
                StatusCode::UNAUTHORIZED,
                "INVALID_CREDENTIALS",
                "Invalid email or password",
            ),
            ApplicationError::TooManyAttempts {
                seconds_until_unlock,
            } => AppError::custom(
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_ATTEMPTS",
                format!(
                    "Account locked. Try again in {} seconds.",
                    seconds_until_unlock
                ),
            ),
            ApplicationError::UserNotFound => {
                AppError::custom(StatusCode::NOT_FOUND, "USER_NOT_FOUND", "User not found")
            }
            ApplicationError::Unexpected(_) => AppError::InternalServerError,
            ApplicationError::UserNotActive => AppError::custom(
                StatusCode::UNAUTHORIZED,
                "USER_NOT_ACTIVE",
                "User is not active. Please contact support.",
            ),
            ApplicationError::InvalidFile(msg) => {
                AppError::custom(StatusCode::BAD_REQUEST, "INVALID_FILE", msg)
            }
            ApplicationError::StorageError(msg) => {
                AppError::custom(StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR", msg)
            }
            ApplicationError::JobNotFound => {
                AppError::custom(StatusCode::NOT_FOUND, "JOB_NOT_FOUND", "Job not found")
            }
            ApplicationError::JobNotDraft => AppError::custom(
                StatusCode::BAD_REQUEST,
                "JOB_NOT_DRAFT",
                "Job is not in draft status",
            ),
            ApplicationError::JobNotDone => AppError::custom(
                StatusCode::BAD_REQUEST,
                "JOB_NOT_DONE",
                "Job is not done yet",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    async fn body_to_json(body: Body) -> serde_json::Value {
        let bytes = body.collect().await.expect("body bytes").to_bytes();
        serde_json::from_slice(&bytes).expect("valid json body")
    }

    #[tokio::test]
    async fn error_response_has_consistent_shape() {
        let response = AppError::custom(
            StatusCode::BAD_REQUEST,
            "INVALID_FILE",
            "Not a valid PDF file",
        )
        .into_response();

        let json = body_to_json(response.into_body()).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "INVALID_FILE");
        assert_eq!(json["error"]["message"], "Not a valid PDF file");
        assert!(json["error"].get("details").is_none());
    }

    #[tokio::test]
    async fn validation_error_includes_details() {
        let response = AppError::Validation(vec!["Email is required".into()]).into_response();

        let json = body_to_json(response.into_body()).await;
        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
        assert_eq!(json["error"]["details"][0], "Email is required");
    }
}
