use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::application::auth::{AuthResult, UserProfile};
use crate::domain::user::UserStatus;

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 1, message = "Name is required"))]
    pub name: String,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub status: UserStatus,
    pub approved_by: Option<uuid::Uuid>,
    pub rejected_at: Option<chrono::NaiveDateTime>,
    pub rejected_by: Option<uuid::Uuid>,
    pub rejection_reason: Option<String>,
    pub role: String,
}

impl From<UserProfile> for UserResponse {
    fn from(profile: UserProfile) -> Self {
        Self {
            id: profile.id,
            email: profile.email,
            name: profile.name,
            status: profile.status,
            approved_by: profile.approved_by,
            rejected_at: profile.rejected_at,
            rejected_by: profile.rejected_by,
            rejection_reason: profile.rejection_reason,
            role: profile.role,
        }
    }
}

impl From<AuthResult> for AuthResponse {
    fn from(result: AuthResult) -> Self {
        Self {
            access_token: result.access_token,
            token_type: "Bearer".to_string(),
            expires_in: result.expires_in,
            user: result.user.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::domain::user::{User, UserStatus};

    #[test]
    fn user_response_serializes_approval_status_fields_without_is_approved() {
        let id = Uuid::parse_str("5cf4f7a1-8f02-42f4-bf1a-bc5c982de0d1").unwrap();
        let admin_id = Uuid::parse_str("2b583c11-01c2-4a28-967b-b12f25dbbc33").unwrap();
        let timestamp =
            NaiveDateTime::parse_from_str("2026-05-28 01:02:03", "%Y-%m-%d %H:%M:%S").unwrap();

        let response = UserResponse::from(UserProfile::from(User {
            id,
            name: "Nadia".to_string(),
            email: "nadia@example.com".to_string(),
            password_hash: "hashed".to_string(),
            status: UserStatus::Rejected,
            approved_by: Some(admin_id),
            rejected_at: Some(timestamp),
            rejected_by: Some(admin_id),
            rejection_reason: Some("Incomplete profile".to_string()),
            role: "user".to_string(),
            created_at: timestamp,
            updated_at: timestamp,
        }));

        let payload = serde_json::to_value(response).unwrap();

        assert_eq!(
            payload,
            json!({
                "id": id,
                "email": "nadia@example.com",
                "name": "Nadia",
                "status": "rejected",
                "approved_by": admin_id,
                "rejected_at": "2026-05-28T01:02:03",
                "rejected_by": admin_id,
                "rejection_reason": "Incomplete profile",
                "role": "user"
            })
        );
        assert!(payload.get("is_approved").is_none());
    }
}
