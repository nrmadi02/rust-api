use std::sync::Arc;

use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::domain::activity_log::{ActivityLog, ActivityLogRepository};

pub struct ActivityLogService {
    activity_log_repo: Arc<dyn ActivityLogRepository>,
}

impl ActivityLogService {
    pub fn new(activity_log_repo: Arc<dyn ActivityLogRepository>) -> Self {
        Self { activity_log_repo }
    }

    pub async fn list_my_activity_logs(
        &self,
        user_id: Uuid,
        page: u32,
        per_page: u32,
        action: Option<&str>,
    ) -> Result<(Vec<ActivityLog>, u64), ApplicationError> {
        let page = page.max(1);
        let per_page = per_page.clamp(1, 100);

        self.activity_log_repo
            .find_by_user(user_id, page, per_page, action)
            .await
            .map_err(ApplicationError::Unexpected)
    }
}
