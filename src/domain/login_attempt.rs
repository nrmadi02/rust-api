use chrono::{DateTime, Utc};

pub struct LoginAttempt {
    pub email: String,
    pub failed_count: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

impl LoginAttempt {
    pub fn is_locked(&self) -> bool {
        self.locked_until.map(|t| t > Utc::now()).unwrap_or(false)
    }

    pub fn seconds_until_unlock(&self) -> i64 {
        self.locked_until
            .map(|t| (t - Utc::now()).num_seconds().max(0))
            .unwrap_or(0)
    }
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait LoginAttemptRepository: Send + Sync {
    async fn find(&self, email: &str) -> Result<Option<LoginAttempt>, DynError>;
    async fn upsert_failure(&self, email: &str) -> Result<(), DynError>;
    async fn delete(&self, email: &str) -> Result<(), DynError>;
}
