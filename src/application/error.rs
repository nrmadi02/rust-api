use thiserror::Error;

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("email already registered")]
    EmailAlreadyRegistered,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("too many attempts")]
    TooManyAttempts { seconds_until_unlock: i64 },
    #[error("user not found")]
    UserNotFound,
    #[error(transparent)]
    Unexpected(#[from] DynError),
}

#[derive(Debug)]
struct DisplayError(String);

impl std::fmt::Display for DisplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DisplayError {}

impl ApplicationError {
    pub fn from_display(err: impl std::fmt::Display) -> Self {
        Self::Unexpected(Box::new(DisplayError(err.to_string())))
    }
}
