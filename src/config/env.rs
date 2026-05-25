use serde::Deserialize;
use validator::Validate;

fn default_port() -> u16 {
    3000
}

#[derive(Debug, Deserialize, Validate)]
pub struct Config {
    #[validate(range(min = 1024, max = 65535))]
    pub port: u16,

    #[validate(url)]
    #[validate(length(min = 1, message = "DATABASE_URL is required"))]
    pub database_url: String,

    #[validate(length(min = 32, message = "JWT_SECRET must be at least 32 characters"))]
    pub jwt_secret: String,

    #[validate(range(min = 1, message = "JWT_EXPIRES_IN must be at least 1 second"))]
    pub jwt_expires_in: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: default_port(),
            database_url: "".to_string(),
            jwt_secret: "".to_string(),
            jwt_expires_in: 86400,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, validator::ValidationErrors> {
        dotenvy::dotenv().ok();
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

        let config = Self {
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or_else(default_port),
            database_url: std::env::var("DATABASE_URL")
                .ok()
                .and_then(|url| url.parse().ok())
                .unwrap_or_default(),
            jwt_secret: std::env::var("JWT_SECRET")
                .ok()
                .and_then(|secret| secret.parse().ok())
                .unwrap_or_default(),
            jwt_expires_in: std::env::var("JWT_EXPIRES_IN")
                .ok()
                .and_then(|expires_in| expires_in.parse().ok())
                .unwrap_or_default(),
        };
        config.validate()?;
        Ok(config)
    }
}
