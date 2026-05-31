use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};

use crate::{
    domain::user::UserStatus,
    presentation::{response::error::AppError, state::AppState},
};

pub struct AuthUser {
    pub user_id: uuid::Uuid,
    pub email: String,
}

pub struct ApprovedUser {
    pub user_id: uuid::Uuid,
    pub email: String,
    pub status: UserStatus,
    pub role: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) =
            TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
                .await
                .map_err(|_| AppError::Unauthorized)?;

        let claims = state
            .jwt_service
            .verify(bearer.token())
            .map_err(|_| AppError::Unauthorized)?;

        let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser {
            user_id,
            email: claims.email,
        })
    }
}

impl FromRequestParts<AppState> for ApprovedUser {
    type Rejection = AppError;
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;
        let profile = state.auth_service.get_current_user(auth.user_id).await?;
        if profile.status != UserStatus::Approved {
            return Err(AppError::custom(
                StatusCode::FORBIDDEN,
                "ACCOUNT_NOT_APPROVED",
                "Your account must be approved before accessing this resource",
            ));
        }
        Ok(ApprovedUser {
            user_id: profile.id,
            email: profile.email,
            status: profile.status,
            role: profile.role,
        })
    }
}
