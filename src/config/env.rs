use serde::Deserialize;
use validator::Validate;

fn default_port() -> u16 {
    3000
}

fn parse_env_var<T>(key: &str) -> Option<T>
where
    T: std::str::FromStr,
{
    std::env::var(key).ok().and_then(|v| v.parse().ok())
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

    #[validate(length(min = 1, message = "UNOSERVER_HOST is required"))]
    pub uno_server_host: String,

    #[validate(range(min = 1024, max = 65535))]
    pub uno_server_port: u16,

    #[validate(range(min = 1, message = "UNOSERVER_TIMEOUT_SECS must be at least 1 second"))]
    pub uno_server_timeout_secs: u64,

    #[validate(length(min = 1, message = "STORAGE_BASE_PATH is required"))]
    pub storage_base_path: String,

    #[validate(range(min = 1, message = "MAX_UPLOAD_SIZE_MB must be at least 1 MB"))]
    pub max_upload_size_mb: u64,

    #[validate(range(min = 1, message = "FILE_TTL_HOURS must be at least 1 hour"))]
    pub file_ttl_hours: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: default_port(),
            database_url: "".to_string(),
            jwt_secret: "".to_string(),
            jwt_expires_in: 86400,
            uno_server_host: "127.0.0.1".to_string(),
            uno_server_port: 2003,
            uno_server_timeout_secs: 60,
            storage_base_path: "storage".to_string(),
            max_upload_size_mb: 10,
            file_ttl_hours: 24,
        }
    }
}

impl Config {
    pub fn from_env() -> Result<Self, validator::ValidationErrors> {
        dotenvy::dotenv().ok();
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

        let config = Self {
            port: parse_env_var("PORT").unwrap_or_else(default_port),
            database_url: parse_env_var("DATABASE_URL").unwrap_or_default(),
            jwt_secret: parse_env_var("JWT_SECRET").unwrap_or_default(),
            jwt_expires_in: parse_env_var("JWT_EXPIRES_IN").unwrap_or_default(),
            uno_server_host: parse_env_var("UNOSERVER_HOST").unwrap_or_default(),
            uno_server_port: parse_env_var("UNOSERVER_PORT").unwrap_or_default(),
            uno_server_timeout_secs: parse_env_var("UNOSERVER_TIMEOUT_SECS").unwrap_or_default(),
            storage_base_path: parse_env_var("STORAGE_BASE_PATH").unwrap_or_default(),
            max_upload_size_mb: parse_env_var("MAX_UPLOAD_SIZE_MB").unwrap_or_default(),
            file_ttl_hours: parse_env_var("FILE_TTL_HOURS").unwrap_or_default(),
        };
        config.validate()?;
        Ok(config)
    }
}
