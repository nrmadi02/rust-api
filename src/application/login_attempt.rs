use std::sync::Arc;

use crate::domain::login_attempt::{LoginAttempt, LoginAttemptRepository};

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct LoginAttemptService {
    repo: Arc<dyn LoginAttemptRepository>,
}

impl LoginAttemptService {
    pub fn new(repo: Arc<dyn LoginAttemptRepository>) -> Self {
        Self { repo }
    }

    pub async fn check_locked(&self, email: &str) -> Result<Option<LoginAttempt>, DynError> {
        let attempt = self.repo.find(email).await?;
        Ok(attempt.filter(|a| a.is_locked()))
    }

    pub async fn record_failure(&self, email: &str) -> Result<(), DynError> {
        let mut attempt = self
            .repo
            .find(email)
            .await?
            .unwrap_or_else(|| LoginAttempt::new(email.to_string()));

        attempt.record_failure();
        self.repo.save(&attempt).await
    }

    pub async fn reset(&self, email: &str) -> Result<(), DynError> {
        self.repo.delete(email).await
    }
}
