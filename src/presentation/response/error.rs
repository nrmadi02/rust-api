use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;

use crate::application::error::ApplicationError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("internal server error")]
    InternalServerError,
    #[error("not found")]
    NotFound,
    #[error("bad request")]
    BadRequest,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("validation failed")]
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

#[derive(Debug, Serialize)]
struct ErrorBody {
    success: bool,
    error: ErrorDetail,
}
#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Vec<String>>,
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

        let body = ErrorBody {
            success: false,
            error: ErrorDetail {
                code,
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
        }
    }
}
