use std::sync::Arc;

use uuid::Uuid;

use crate::application::error::ApplicationError;
use crate::application::jwt::JwtService;
use crate::application::login_attempt::LoginAttemptService;
use crate::application::password::{hash_password, verify_password};
use crate::domain::user::{User, UserRepository};

#[derive(Debug)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub name: String,
}

impl From<User> for UserProfile {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            name: user.name,
        }
    }
}

#[derive(Debug)]
pub struct AuthResult {
    pub access_token: String,
    pub expires_in: i64,
    pub user: UserProfile,
}

pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    login_attempt_service: Arc<LoginAttemptService>,
    jwt_service: Arc<JwtService>,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        login_attempt_service: Arc<LoginAttemptService>,
        jwt_service: Arc<JwtService>,
    ) -> Self {
        Self {
            user_repo,
            login_attempt_service,
            jwt_service,
        }
    }

    pub async fn register(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<AuthResult, ApplicationError> {
        if self.user_repo.find_by_email(email).await?.is_some() {
            return Err(ApplicationError::EmailAlreadyRegistered);
        }

        let password_hash = hash_password(password).map_err(ApplicationError::from_display)?;
        let user = self.user_repo.create(name, email, &password_hash).await?;

        let token = self
            .jwt_service
            .generate(user.id, &user.email)
            .map_err(ApplicationError::from_display)?;

        Ok(AuthResult {
            access_token: token,
            expires_in: self.jwt_service.expires_in(),
            user: user.into(),
        })
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<AuthResult, ApplicationError> {
        if let Some(attempt) = self.login_attempt_service.check_locked(email).await? {
            return Err(ApplicationError::TooManyAttempts {
                seconds_until_unlock: attempt.seconds_until_unlock(),
            });
        }

        let user = self.user_repo.find_by_email(email).await?;

        let Some(user) = user else {
            return Err(ApplicationError::InvalidCredentials);
        };

        let password_valid = verify_password(password, &user.password_hash)
            .map_err(ApplicationError::from_display)?;

        if !password_valid {
            self.login_attempt_service.record_failure(email).await?;
            return Err(ApplicationError::InvalidCredentials);
        }

        self.login_attempt_service.reset(email).await?;

        let token = self
            .jwt_service
            .generate(user.id, &user.email)
            .map_err(ApplicationError::from_display)?;

        Ok(AuthResult {
            access_token: token,
            expires_in: self.jwt_service.expires_in(),
            user: user.into(),
        })
    }

    pub async fn get_current_user(&self, user_id: Uuid) -> Result<UserProfile, ApplicationError> {
        self.user_repo
            .find_by_id(user_id)
            .await?
            .map(UserProfile::from)
            .ok_or(ApplicationError::UserNotFound)
    }
}
