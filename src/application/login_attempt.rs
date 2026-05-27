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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{Duration, Utc};

    use super::*;
    use crate::domain::login_attempt::{LoginAttempt, MAX_FAILED_ATTEMPTS};

    struct InMemoryLoginAttemptRepository {
        store: Mutex<HashMap<String, LoginAttempt>>,
    }

    impl InMemoryLoginAttemptRepository {
        fn new() -> Self {
            Self {
                store: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl LoginAttemptRepository for InMemoryLoginAttemptRepository {
        async fn find(&self, email: &str) -> Result<Option<LoginAttempt>, DynError> {
            Ok(self
                .store
                .lock()
                .unwrap()
                .get(email)
                .cloned()
                .map(|mut attempt| {
                    attempt.email = email.to_string();
                    attempt
                }))
        }

        async fn save(&self, attempt: &LoginAttempt) -> Result<(), DynError> {
            self.store.lock().unwrap().insert(
                attempt.email.clone(),
                LoginAttempt {
                    email: attempt.email.clone(),
                    failed_count: attempt.failed_count,
                    locked_until: attempt.locked_until,
                },
            );
            Ok(())
        }

        async fn delete(&self, email: &str) -> Result<(), DynError> {
            self.store.lock().unwrap().remove(email);
            Ok(())
        }
    }

    #[tokio::test]
    async fn locks_account_after_max_failed_attempts() {
        let repo = Arc::new(InMemoryLoginAttemptRepository::new());
        let service = LoginAttemptService::new(repo.clone());
        let email = "user@example.com";

        for _ in 0..MAX_FAILED_ATTEMPTS {
            service.record_failure(email).await.unwrap();
        }

        let locked = service.check_locked(email).await.unwrap();
        assert!(locked.is_some());
        assert!(locked.unwrap().is_locked());
    }

    #[tokio::test]
    async fn reset_clears_lockout_state() {
        let repo = Arc::new(InMemoryLoginAttemptRepository::new());
        let service = LoginAttemptService::new(repo.clone());
        let email = "user@example.com";

        repo.save(&LoginAttempt {
            email: email.to_string(),
            failed_count: MAX_FAILED_ATTEMPTS,
            locked_until: Some(Utc::now() + Duration::minutes(15)),
        })
        .await
        .unwrap();

        service.reset(email).await.unwrap();

        assert!(service.check_locked(email).await.unwrap().is_none());
    }
}
