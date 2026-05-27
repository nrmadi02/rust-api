use chrono::{DateTime, Duration, Utc};

pub const MAX_FAILED_ATTEMPTS: i32 = 5;
pub const LOCK_DURATION_MINUTES: i64 = 15;

#[derive(Clone)]
pub struct LoginAttempt {
    pub email: String,
    pub failed_count: i32,
    pub locked_until: Option<DateTime<Utc>>,
}

impl LoginAttempt {
    pub fn new(email: String) -> Self {
        Self {
            email,
            failed_count: 0,
            locked_until: None,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked_until.map(|t| t > Utc::now()).unwrap_or(false)
    }

    pub fn seconds_until_unlock(&self) -> i64 {
        self.locked_until
            .map(|t| (t - Utc::now()).num_seconds().max(0))
            .unwrap_or(0)
    }

    pub fn record_failure(&mut self) {
        self.failed_count += 1;
        if self.failed_count >= MAX_FAILED_ATTEMPTS {
            self.locked_until = Some(Utc::now() + Duration::minutes(LOCK_DURATION_MINUTES));
        }
    }
}

type DynError = Box<dyn std::error::Error + Send + Sync>;

#[async_trait::async_trait]
pub trait LoginAttemptRepository: Send + Sync {
    async fn find(&self, email: &str) -> Result<Option<LoginAttempt>, DynError>;
    async fn save(&self, attempt: &LoginAttempt) -> Result<(), DynError>;
    async fn delete(&self, email: &str) -> Result<(), DynError>;
}
